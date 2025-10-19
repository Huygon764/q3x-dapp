mod batch;
mod chain_fusion;
mod ecdsa;
mod evm_types;
mod privacy;
mod vetkey;
mod wallet;

use crate::batch::{execute_batch, propose_batch, transfer_icp, BatchTransaction};
use crate::privacy::{
    decrypt_wallet_data, encrypt_message_with_vetkeys, encrypt_principals_with_vetkeys,
};
use crate::vetkey::set_vetkey_id;
use crate::wallet::{MultiSignatureWallet, TransferArgs, Wallet, WalletError};
use chain_fusion::*;
use ic_cdk::api::management_canister::ecdsa::EcdsaKeyId;
use ic_cdk::{init, query, update};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
use std::str::FromStr;

use crate::ecdsa::{get_ecdsa_key_id_from_env, is_signature_valid, sign_message};

use candid::Principal;

use ic_ledger_types::Tokens;

type WalletStore = BTreeMap<String, Wallet>;
type PrincipalWalletsMap = BTreeMap<Principal, HashSet<String>>; // use HashSet because we don't want duplicate wallet ids

thread_local! {
    static PRINCIPAL_WALLETS_MAP: RefCell<PrincipalWalletsMap> = RefCell::default();
    static WALLETS: RefCell<WalletStore> = RefCell::default();
    static KEY_ID: RefCell<EcdsaKeyId> = RefCell::default();
}

const WALLET_NOT_FOUND_ERROR: &str = "WalletNotFound";
const WALLET_ALREADY_EXISTS_ERROR: &str = "WalletAlreadyExists";
const WALLET_MSG_ALREADY_QUEUED_ERROR: &str = "WalletMsgAlreadyQueued";
const WALLET_INVALID_SIGNATURE_ERROR: &str = "WalletInvalidSignature";
const WALLET_CANNOT_SIGN_ERROR: &str = "WalletCannotSign";
const WALLET_SIGNERS_NOT_MATCH_THRESHOLD: &str = "WalletSignersNotMatchThreshold";
const METADATA_NOT_FOUND: &str = "MetadataNotFound";
const ENCRYPTION_ERROR: &str = "EncryptionError";

/// Initializes the module with environment-specific configurations.
///
/// # Arguments
///
/// * `env` - A string representing the environment.
///
/// # Behavior
///
/// Initializes the KEY_ID with an EcdsaKeyId based on the provided environment.
/// Initializes VetKD key ID for encryption/decryption operations based on the provided environment.
#[init]
fn init(env: String) {
    KEY_ID.with(|key_id| {
        key_id
            .borrow_mut()
            .clone_from(&get_ecdsa_key_id_from_env(&env));
    });

    // Initialize VetKD key for encryption/decryption
    set_vetkey_id(&env);
}

/// Creates a new wallet.
///
/// # Arguments
///
/// * `wallet_id` - Unique identifier for the wallet as a String.
/// * `signers` - A list of Principals representing the signers of the wallet.
/// * `threshold` - The threshold number of signers required for a transaction.
///
/// # Returns
///
/// * `Result<(), String>` - Result indicating success or an error message.
#[update]
async fn create_wallet(
    wallet_id: String,
    signers: Vec<Principal>,
    threshold: u8,
) -> Result<(), String> {
    if WALLETS.with(|wallets| wallets.borrow().contains_key(&wallet_id)) {
        return Err(WALLET_ALREADY_EXISTS_ERROR.to_string());
    }

    let encrypted_signers = encrypt_principals_with_vetkeys(&signers, &wallet_id)
        .await
        .map_err(|e| format!("Failed to encrypt signers: {}", e))?;

    let mut wallet = Wallet::default();
    encrypted_signers.iter().for_each(|signer| {
        wallet.add_signer_principal_hex(signer.clone());
    });

    if wallet.set_default_threshold(threshold).is_err() {
        return Err(WALLET_SIGNERS_NOT_MATCH_THRESHOLD.to_string());
    }

    let wallet_id_clone = wallet_id.clone(); // Clone wallet_id
    WALLETS.with(|wallets| {
        wallets.borrow_mut().insert(wallet_id_clone, wallet.clone()); // Clone wallet
    });

    // Now, use the original wallet and wallet_id
    for signer in signers {
        let wallet_id_clone = wallet_id.clone(); // Clone wallet_id for use in the closure
        PRINCIPAL_WALLETS_MAP.with(|map| {
            let mut map = map.borrow_mut();
            map.entry(signer).or_default().insert(wallet_id_clone);
        });
    }
    Ok(())
}

/// Retrieves a wallet by its ID.
///
/// # Arguments
///
/// * `wallet_id` - The unique identifier for the wallet as a String.
///
/// # Returns
///
/// * `Option<Wallet>` - The wallet if found, otherwise None.
#[update]
async fn get_wallet(wallet_id: String) -> Option<Wallet> {
    let wallet = WALLETS.with(|wallets| wallets.borrow().get(&wallet_id).cloned());

    if let Some(wallet) = wallet {
        if wallet.has_signer(&wallet_id).await {
            // Authorized - return decrypted wallet
            match decrypt_wallet_data(
                &wallet.get_signers(),
                &wallet.get_default_threshold(),
                &wallet.get_encrypted_messages_with_signers(),
                &wallet.get_all_metadata(),
                &wallet_id,
            )
            .await
            {
                Ok(decrypted_wallet) => Some(decrypted_wallet),
                Err(_) => Some(wallet),
            }
        } else {
            // Not authorized - return encrypted wallet
            Some(wallet)
        }
    } else {
        // Wallet not found
        None
    }
}

/// Proposes a message to be signed by the wallet.
///
/// # Arguments
///
/// * `wallet_id` - The wallet's unique identifier.
/// * `msg` - The message to be proposed, in hexadecimal format.
///
/// # Returns
///
/// * `Result<(), String>` - Result indicating success or an error message.
#[update]
async fn propose(wallet_id: String, msg: String) -> Result<(), String> {
    debug_println_caller("propose");
    let msg = hex::decode(msg).map_err(|_| "InvalidMessage".to_string())?;

    let mut wallet = WALLETS
        .with(|wallets| wallets.borrow().get(&wallet_id).cloned())
        .ok_or_else(|| WALLET_NOT_FOUND_ERROR.to_string())?;

    ic_cdk::println!(
        "Proposing message of length {} to wallet {}",
        msg.len(),
        wallet_id
    );

    wallet
        .propose_message(msg, &wallet_id)
        .await
        .map_err(|error| match error {
            WalletError::InvalidSignature => WALLET_INVALID_SIGNATURE_ERROR.to_string(),
            WalletError::MsgAlreadyQueued => WALLET_MSG_ALREADY_QUEUED_ERROR.to_string(),
            WalletError::EncryptionError => ENCRYPTION_ERROR.to_string(),
            _ => "UnknownError".to_string(),
        })?;

    WALLETS.with(|wallets| {
        wallets.borrow_mut().insert(wallet_id, wallet);
    });

    Ok(())
}

/// Checks if a message can be signed by the wallet.
///
/// # Arguments
///
/// * `wallet_id` - The wallet's unique identifier.
/// * `msg` - The message to be checked, in hexadecimal format.
///
/// # Returns
///
/// * `bool` - True if the message can be signed, otherwise false.
#[update]
async fn can_sign(wallet_id: String, msg: String) -> bool {
    let decode_msg = hex::decode(&msg).unwrap_or_default();
    let encrypted_message = match encrypt_message_with_vetkeys(&decode_msg, &wallet_id).await {
        Ok(msg) => msg,
        Err(_) => return false,
    };

    WALLETS.with(|wallets| {
        wallets
            .borrow()
            .get(&wallet_id)
            .ok_or(WALLET_NOT_FOUND_ERROR.to_string())
            .unwrap()
            .can_sign(&encrypted_message)
    })
}

/// Approves a message for signing in the wallet.
///
/// # Arguments
///
/// * `wallet_id` - The wallet's unique identifier.
/// * `msg` - The message to be approved, in hexadecimal format.
///
/// # Returns
///
/// * `Result<u8, String>` - The number of signatures or an error message.
#[update]
async fn approve(wallet_id: String, msg: String) -> Result<u8, String> {
    debug_println_caller("approve");
    let msg = hex::decode(msg).map_err(|_| "InvalidMessage".to_string())?;

    let mut wallet = WALLETS
        .with(|wallets| wallets.borrow().get(&wallet_id).cloned())
        .ok_or_else(|| WALLET_NOT_FOUND_ERROR.to_string())?;

    let result = wallet
        .approve(msg, &wallet_id)
        .await
        .map_err(|error| match error {
            WalletError::MsgNotQueued => "WalletMsgNotQueued".to_string(),
            WalletError::InvalidSignature => WALLET_INVALID_SIGNATURE_ERROR.to_string(),
            WalletError::MsgAlreadySignedBySigner => "WalletMsgAlreadySignedBySigner".to_string(),
            _ => "UnknownError".to_string(),
        })?;

    WALLETS.with(|wallets| {
        wallets.borrow_mut().insert(wallet_id, wallet);
    });

    Ok(result)
}

/// Signs a message using the wallet.
///
/// # Arguments
///
/// * `wallet_id` - The wallet's unique identifier.
/// * `msg` - The message to be signed, in hexadecimal format.
///
/// # Returns
///
/// * `Result<String, String>` - The signature in hexadecimal format or an error message.
#[update]
async fn sign(wallet_id: String, msg: String) -> Result<String, String> {
    let msg: Vec<u8> = hex::decode(msg).map_err(|_| "InvalidMessage".to_string())?;

    let encrypted_message = encrypt_message_with_vetkeys(&msg, &wallet_id)
        .await
        .map_err(|_| "EncryptionError".to_string())?;

    let can_sign = WALLETS.with(|wallets| {
        wallets
            .borrow()
            .get(&wallet_id)
            .ok_or(WALLET_NOT_FOUND_ERROR.to_string())
            .unwrap()
            .can_sign(&encrypted_message)
    });

    let key_id = KEY_ID.with(|key_id| key_id.borrow().clone());

    if !can_sign {
        return Err(WALLET_CANNOT_SIGN_ERROR.to_string());
    }

    // Try batch format first
    // if let Ok(batch) = candid::decode_one::<BatchTransaction>(&msg) {
    //     ic_cdk::println!("🎯 Processing batch transaction: {}", batch.id);

    //     let batch_result = execute_batch(wallet_id.clone(), batch).await;

    //     // Remove message from pending after execution
    //     let mut wallet = WALLETS
    //         .with(|wallets| wallets.borrow().get(&wallet_id).cloned())
    //         .ok_or_else(|| WALLET_NOT_FOUND_ERROR.to_string())?;

    //     let _ = wallet
    //         .remove_message_and_metadata(msg.clone(), &wallet_id)
    //         .await;

    //     WALLETS.with(|wallets| {
    //         wallets.borrow_mut().insert(wallet_id, wallet);
    //     });

    //     return batch_result;
    // }

    let mut is_special_message = false;
    let mut pending_batch: Option<BatchTransaction> = None;
    let mut pending_evm_transfer: Option<(Wallet, TransferEvmArgs)> = None;
    let mut pending_transfer: Option<(Wallet, TransferArgs)> = None;
    let mut pending_add_signer: Option<Principal> = None;
    let mut pending_remove_signer: Option<Principal> = None;

    if let Ok(message_str) = String::from_utf8(msg.clone()) {
        WALLETS.with(|wallets| {
            let mut wallets = wallets.borrow_mut();

            // it is safe to unwrap here, as we checked that the wallet exists before
            let wallet = wallets
                .get_mut(&wallet_id)
                .ok_or(WALLET_NOT_FOUND_ERROR.to_string())
                .unwrap();

            // Handle special add/remove signer commands
            if message_str.starts_with("ADD_SIGNER::") {
                let new_signer_str = &message_str["ADD_SIGNER::".len()..];
                if let Ok(new_signer) = Principal::from_str(new_signer_str) {
                    pending_add_signer = Some(new_signer);
                }
                is_special_message = true;
            } else if message_str.starts_with("REMOVE_SIGNER::") {
                let signer_to_remove_str = &message_str["REMOVE_SIGNER::".len()..];
                if let Ok(signer_to_remove) = Principal::from_str(signer_to_remove_str) {
                    pending_remove_signer = Some(signer_to_remove);
                }
                is_special_message = true;
            }
            // Handle special set threshold command
            else if message_str.starts_with("SET_THRESHOLD::") {
                let new_threshold_str = &message_str["SET_THRESHOLD::".len()..];
                if let Ok(new_threshold) = u8::from_str(new_threshold_str) {
                    wallet.set_default_threshold(new_threshold).unwrap();
                }
                is_special_message = true;
            }
            // Handle special transfer command
            else if message_str.starts_with("TRANSFER::") {
                // split the string: "TRANSFER::1000000000000000000::principal_id"
                let parts: Vec<&str> = message_str.split("::").collect();
                ic_cdk::println!("{:#?}", parts);
                let amount = Tokens::from_e8s(parts[1].parse::<u64>().unwrap());
                let to_principal = Principal::from_str(parts[2]).unwrap();
                let to_subaccount = None; // FixMe: add subaccount
                let transfer_args = TransferArgs {
                    amount,
                    to_principal,
                    to_subaccount,
                };
                pending_transfer = Some((wallet.clone(), transfer_args));
                is_special_message = true;
            } else if message_str.starts_with("TRANSFER_EVM::") {
                ic_cdk::println!("🔍 Processing EVM transfer message");
                let parts: Vec<&str> = message_str.split("::").collect();

                if parts.len() != 6 {
                    ic_cdk::println!("❌ Invalid EVM transfer message format");
                } else {
                    ic_cdk::println!("📋 EVM transfer parts: {:?}", parts);

                    // Parse the parts
                    let to = parts[1].to_string();
                    let value = parts[2].parse::<u128>().unwrap_or(0);
                    let chain_id = parts[3].parse::<u64>().unwrap_or(0);
                    let gas_price = parts[4].parse::<u128>().unwrap_or(0);
                    let gas_limit = parts[5].parse::<u64>().unwrap_or(0);

                    let evm_transfer_args = TransferEvmArgs {
                        wallet_id: wallet_id.clone(),
                        to,
                        value,
                        chain_id,
                        gas_price,
                        gas_limit,
                    };

                    ic_cdk::println!("⚡ Parsed EVM args: {:?}", evm_transfer_args);
                    pending_evm_transfer = Some((wallet.clone(), evm_transfer_args));
                    is_special_message = true;
                }
            } else if message_str.starts_with("BATCH::") {
                ic_cdk::println!("🔄 Processing batch transaction");

                let parts: Vec<&str> = message_str.split("::").collect();
                if parts.len() == 3 {
                    let batch_id = parts[1];
                    let encoded_data = parts[2];

                    // Decode from hex
                    if let Ok(candid_bytes) = hex::decode(encoded_data) {
                        // Deserialize BatchTransaction
                        if let Ok(batch) = candid::decode_one::<BatchTransaction>(&candid_bytes) {
                            ic_cdk::println!("📋 Prepared batch for execution: {}", batch.id);
                            // Store batch for execution outside closure
                            pending_batch = Some(batch);
                        }
                    }
                }
                is_special_message = true;
            }
        });
    }

    // handle for add and remove signer
    if let Some(new_signer) = pending_add_signer {
        if let Some(mut wallet) = WALLETS.with(|w| w.borrow().get(&wallet_id).cloned()) {
            if wallet.add_signer(new_signer, &wallet_id).await.is_ok() {
                WALLETS.with(|w| w.borrow_mut().insert(wallet_id.clone(), wallet));
                PRINCIPAL_WALLETS_MAP.with(|map| {
                    map.borrow_mut()
                        .entry(new_signer)
                        .or_default()
                        .insert(wallet_id.clone());
                });
            }
        }
    }
    if let Some(signer_to_remove) = pending_remove_signer {
        if let Some(mut wallet) = WALLETS.with(|w| w.borrow().get(&wallet_id).cloned()) {
            if wallet
                .remove_signer(signer_to_remove, &wallet_id)
                .await
                .is_ok()
            {
                WALLETS.with(|w| w.borrow_mut().insert(wallet_id.clone(), wallet));
                PRINCIPAL_WALLETS_MAP.with(|map| {
                    if let Some(wallet_list) = map.borrow_mut().get_mut(&signer_to_remove) {
                        wallet_list.retain(|id| id != &wallet_id);
                    }
                });
            }
        }
    }

    // Handle EVM transfer execution
    if let Some((wallet_clone, evm_args)) = pending_evm_transfer {
        ic_cdk::println!("🚀 Executing EVM transfer with multisig approval");

        match chain_fusion::transfer_evm(evm_args, None).await {
            Ok(tx_hash) => {
                ic_cdk::println!("✅ EVM Transfer completed: {}", tx_hash);
            }
            Err(e) => {
                ic_cdk::println!("❌ EVM Transfer failed: {}", e);
                return Err(format!("EVM Transfer failed: {}", e));
            }
        }
    }

    // handle for transfer
    if let Some((wallet_clone, args)) = pending_transfer {
        transfer_icp(args)
            .await
            .map_err(|e| format!("Transfer failed: {e}"))?;
    }

    // batch process
    if let Some(batch) = pending_batch {
        ic_cdk::println!("🚀 Executing batch transaction: {}", batch.id);

        match execute_batch(wallet_id.clone(), batch).await {
            Ok(_) => {
                ic_cdk::println!("✅ Batch executed successfully");
            }
            Err(e) => {
                ic_cdk::println!("❌ Batch execution failed: {}", e);
                return Err(format!("Batch execution failed: {}", e));
            }
        }
    }

    let signature = match is_special_message {
        true => "".to_string(),
        false => hex::encode(sign_message(wallet_id.clone(), msg.clone(), key_id).await?),
    };

    let mut wallet = WALLETS
        .with(|wallets| wallets.borrow().get(&wallet_id).cloned())
        .ok_or_else(|| WALLET_NOT_FOUND_ERROR.to_string())?;

    let _ = wallet
        .remove_message_and_metadata(msg.clone(), &wallet_id)
        .await;

    WALLETS.with(|wallets| {
        wallets.borrow_mut().insert(wallet_id, wallet);
    });

    Ok(signature)
}

/// Verifies a signature for a given message and wallet.
///
/// # Arguments
///
/// * `wallet_id` - The wallet's unique identifier.
/// * `message` - The message associated with the signature, in hexadecimal format.
/// * `signature` - The signature to be verified, in hexadecimal format.
///
/// # Returns
///
/// * `Result<bool, String>` - True if the signature is valid, otherwise an error message.
#[update]
async fn verify_signature(
    wallet_id: String,
    message: String,
    signature: String,
) -> Result<bool, String> {
    let message = hex::decode(message).map_err(|_| "Invalid message".to_string())?;
    let signature = hex::decode(signature).map_err(|_| "Invalid signature".to_string())?;
    let key_id = KEY_ID.with(|key_id| key_id.borrow().clone());
    is_signature_valid(message, signature, wallet_id, key_id).await
}

/// Retrieves all messages that can be signed for a given wallet.
///
/// # Arguments
///
/// * `wallet_id` - The wallet's unique identifier.
///
/// # Returns
///
/// * `Vec<Vec<u8>>` - A list of messages that can be signed.
#[update]
async fn get_messages_to_sign(wallet_id: String) -> Result<Vec<String>, String> {
    let wallet = WALLETS
        .with(|wallets| wallets.borrow().get(&wallet_id).cloned())
        .ok_or_else(|| WALLET_NOT_FOUND_ERROR.to_string())?;

    let messages = wallet.get_messages_to_sign(&wallet_id).await?;

    Ok(messages.into_iter().map(hex::encode).collect())
}

/// Retrieves all messages that have been proposed for a given wallet.
///
/// # Arguments
///
/// * `wallet_id` - The wallet's unique identifier.
///
/// # Returns
///
/// * `Vec<Vec<u8>>` - A list of messages that have been proposed.
#[update]
async fn get_proposed_messages(wallet_id: String) -> Result<Vec<String>, String> {
    let wallet = WALLETS
        .with(|wallets| wallets.borrow().get(&wallet_id).cloned())
        .ok_or_else(|| WALLET_NOT_FOUND_ERROR.to_string())?;

    let messages = wallet.get_proposed_messages(&wallet_id).await?;

    Ok(messages.into_iter().map(hex::encode).collect())
}

/// Retrieves all messages that have been proposed along with their signers for a given wallet.
///
/// # Arguments
///
/// * `wallet_id` - The wallet's unique identifier.
///
/// # Returns
///
/// * `Vec<(Vec<u8>, Vec<String>)>` - A list of tuples containing messages and their signers (hex-string).
#[update]
async fn get_messages_with_signers(
    wallet_id: String,
) -> Result<Vec<(String, Vec<String>)>, String> {
    let wallet = WALLETS
        .with(|wallets| wallets.borrow().get(&wallet_id).cloned())
        .ok_or_else(|| WALLET_NOT_FOUND_ERROR.to_string())?;

    let messages_with_signers = wallet.get_messages_with_signers(&wallet_id).await?;

    let result = messages_with_signers
        .into_iter()
        .map(|(msg, signers)| (hex::encode(msg), signers))
        .collect();

    Ok(result)
}

/// Proposes adding a new signer to the wallet.
///
/// # Arguments
///
/// * `wallet_id` - The wallet's unique identifier.
/// * `new_signer` - The Principal of the new signer to add.
///
/// # Returns
///
/// * `Result<(), String>` - Result indicating success or an error message.
#[update]
async fn add_signer(wallet_id: String, new_signer: Principal) -> Result<String, String> {
    debug_println_caller("add_signer");
    let special_message = hex::encode(format!("ADD_SIGNER::{new_signer}"));
    let _ = propose(wallet_id, special_message.clone()).await?;
    Ok(special_message)
}

/// Proposes removing a signer from the wallet.
///
/// # Arguments
///
/// * `wallet_id` - The wallet's unique identifier.
/// * `signer_to_remove` - The Principal of the signer to remove.
///
/// # Returns
///
/// * `Result<(), String>` - Result indicating success or an error message.
#[update]
async fn remove_signer(wallet_id: String, signer_to_remove: Principal) -> Result<String, String> {
    debug_println_caller("remove_signer");
    let special_message = hex::encode(format!("REMOVE_SIGNER::{signer_to_remove}"));
    let _ = propose(wallet_id, special_message.clone()).await?;
    Ok(special_message)
}

/// Proposes setting a new threshold for the wallet.
///
/// # Arguments
///
/// * `wallet_id` - The wallet's unique identifier.
/// * `new_threshold` - The new threshold value to set.
///
/// # Returns
///
/// * `Result<String, String>` - Result indicating success or an error message.
#[update]
async fn set_threshold(wallet_id: String, new_threshold: u8) -> Result<String, String> {
    let special_message = hex::encode(format!("SET_THRESHOLD::{new_threshold}"));
    let _ = propose(wallet_id, special_message.clone()).await?;
    Ok(special_message)
}

#[update]
async fn transfer(
    wallet_id: String,
    amount: u64,
    to_principal: Principal,
) -> Result<String, String> {
    let special_message = hex::encode(format!("TRANSFER::{amount}::{to_principal}"));
    let _ = propose(wallet_id, special_message.clone()).await?;
    Ok(special_message)
}

/// Retrieves all wallets associated with a given principal.
///
/// # Arguments
///
/// * `principal` - The principal to retrieve wallets for.
///
/// # Returns
///
/// * `HashSet<String>` - A list of wallet IDs associated with the principal.
#[query]
fn get_wallets_for_principal(principal: Principal) -> HashSet<String> {
    PRINCIPAL_WALLETS_MAP.with(|map| map.borrow().get(&principal).cloned().unwrap_or_default())
}

/// Add metadata to a message in the wallet.
///
/// * `message` - The message as a `Vec<u8>`.
/// * `metadata` - The metadata as a `String`.
/// * `caller` - The `Principal` of the caller.
///
/// Returns `Result<(), String>` indicating success or the type of failure.
#[update]
async fn add_metadata(wallet_id: String, msg: String, metadata: String) -> Result<(), String> {
    let msg = hex::decode(msg).map_err(|_| "InvalidMessage".to_string())?;

    let mut wallet = WALLETS
        .with(|wallets| wallets.borrow().get(&wallet_id).cloned())
        .ok_or_else(|| WALLET_NOT_FOUND_ERROR.to_string())?;

    wallet.add_metadata(msg, metadata, &wallet_id).await?;

    WALLETS.with(|wallets| {
        wallets.borrow_mut().insert(wallet_id, wallet);
    });

    Ok(())
}

/// Get the metadata associated with a message in the wallet.
///
/// * `message` - The message as a `Vec<u8>`.
///
/// Returns `Option<&String>` containing the metadata if it exists.
#[update]
async fn get_metadata(wallet_id: String, msg: String) -> Result<String, String> {
    let msg = hex::decode(msg).map_err(|_| "InvalidMessage".to_string())?;

    let wallet = WALLETS
        .with(|wallets| wallets.borrow().get(&wallet_id).cloned())
        .ok_or_else(|| WALLET_NOT_FOUND_ERROR.to_string())?;

    let result = wallet
        .get_metadata(msg, &wallet_id)
        .await
        .ok_or_else(|| METADATA_NOT_FOUND.to_string())?;

    Ok(result.clone())
}

/// Proposes a message and adds metadata in one call.
///
/// # Arguments
///
/// * `wallet_id` - The wallet's unique identifier.
/// * `msg` - The message to be proposed, in hexadecimal format.
/// * `metadata` - The metadata to be added to the message.
///
/// # Returns
///
/// * `Result<(), String>` - Result indicating success or an error message.
#[update]
async fn propose_with_metadata(
    wallet_id: String,
    msg: String,
    metadata: String,
) -> Result<(), String> {
    propose(wallet_id.clone(), msg.clone()).await?;
    add_metadata(wallet_id, msg, metadata).await
}

#[update]
pub async fn get_evm_address(wallet_id: String) -> Result<String, String> {
    chain_fusion::get_evm_address(wallet_id).await
}

// #[update]
// pub async fn transfer_evm(args: TransferEvmArgs) -> Result<String, String> {
//     chain_fusion::transfer_evm(args).await
// }

#[update]
async fn transfer_evm(args: TransferEvmArgs) -> Result<String, String> {
    ic_cdk::println!("🚀 Creating EVM transfer proposal");
    ic_cdk::println!("Wallet: {}", args.wallet_id);
    ic_cdk::println!("To: {}", args.to);
    ic_cdk::println!("Value: {} Wei", args.value);
    ic_cdk::println!("Chain: {}", args.chain_id);
    ic_cdk::println!("Gas Price: {} Wei", args.gas_price);
    ic_cdk::println!("Gas Limit: {}", args.gas_limit);

    // Validate inputs
    if args.to.is_empty() || !args.to.starts_with("0x") || args.to.len() != 42 {
        return Err("Invalid recipient address".to_string());
    }

    // Create special EVM message
    let special_message = hex::encode(format!(
        "TRANSFER_EVM::{}::{}::{}::{}::{}",
        args.to, args.value, args.chain_id, args.gas_price, args.gas_limit
    ));

    ic_cdk::println!("📝 Special message: {}", special_message);

    // Propose the message (same as ICP transfers)
    let _ = propose(args.wallet_id, special_message.clone()).await?;

    Ok(special_message)
}

#[update]
pub async fn get_transaction_count(wallet_id: String, chain_id: u64) -> Result<u64, String> {
    chain_fusion::get_transaction_count(wallet_id, chain_id).await
}

#[update]
pub async fn get_icp_balance(wallet_id: String) -> Result<ICPBalance, String> {
    chain_fusion::get_icp_balance(wallet_id).await
}

#[update]
pub async fn get_wallet_portfolio(
    wallet_id: String,
    chain_id: u64,
) -> Result<PortfolioBalance, String> {
    chain_fusion::get_wallet_portfolio(wallet_id, chain_id).await
}

#[update]
pub async fn propose_batch_transaction(
    wallet_id: String,
    batch: BatchTransaction,
) -> Result<String, String> {
    propose_batch(wallet_id, batch).await
}
// pub async fn propose_batch_transaction(
//     wallet_id: String,
//     batch: BatchTransaction,
// ) -> Result<String, String> {
//     // Serialize BatchTransaction to Candid bytes
//     let candid_bytes =
//         candid::encode_one(&batch).map_err(|e| format!("Failed to encode batch: {}", e))?;

//     // Encode to hex for safe string transmission
//     let encoded_batch = hex::encode(&candid_bytes);

//     // Create special message pattern
//     let special_message = hex::encode(format!("BATCH::{}::{}", batch.id, encoded_batch));

//     ic_cdk::println!("📦 Creating batch proposal: {}", batch.id);

//     // Propose
//     let _ = propose(wallet_id, special_message.clone()).await?;

//     Ok(special_message)
// }

fn debug_println_caller(method_name: &str) {
    ic_cdk::println!(
        "{}: caller: {} (isAnonymous: {})",
        method_name,
        ic_cdk::caller().to_text(),
        ic_cdk::caller() == Principal::anonymous()
    );
}

// Enable Candid export
ic_cdk::export_candid!();
