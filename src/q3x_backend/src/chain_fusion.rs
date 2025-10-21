use std::vec;

use crate::{ecdsa::get_derivation_path, evm_types::*, wallet};
use candid::{CandidType, Principal};
use crc32fast::Hasher as Crc32Hasher;
use ethereum_tx_sign::{EcdsaSig, LegacyTransaction, Transaction};
use hex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha224};
use tiny_keccak::{Hasher, Keccak};

use crate::ecdsa::{get_ecdsa_key_id_from_env, get_public_key, sign_message};

// const EVM_CANISTER_ID: &str = "u6s2n-gx777-77774-qaaba-cai";
const EVM_CANISTER_ID: &str = "7hfb6-caaaa-aaaar-qadga-cai";
const LEDGER_CANISTER_ID: &str = "ryjl3-tyaaa-aaaaa-aaaba-cai";

#[derive(candid::CandidType, serde::Deserialize, Debug)]
pub struct TransferEvmArgs {
    pub wallet_id: String,
    pub to: String,
    pub value: u128,
    pub chain_id: u64,
    pub gas_price: u128,
    pub gas_limit: u64,
}

#[derive(CandidType, Serialize, Deserialize, Debug, Clone)]
pub struct TokenBalance {
    pub symbol: String,
    pub contract_address: String,
    pub balance: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(CandidType, Serialize, Deserialize, Debug, Clone)]
pub struct PortfolioBalance {
    pub icp_balance: ICPBalance,
    pub native_balance: String,
    pub native_symbol: String, // "ETH", "BNB", etc.
    pub token_balances: Vec<TokenBalance>,
    pub wallet_address: String,
    pub chain_id: u64,
}

#[derive(CandidType, Deserialize, Debug)]
pub struct AccountBalanceArgs {
    pub account: Vec<u8>,
}

#[derive(CandidType, Deserialize, Debug)]
pub struct Tokens {
    pub e8s: u64,
}

#[derive(CandidType, Serialize, Deserialize, Debug, Clone)]
pub struct ICPBalance {
    pub balance_e8s: u64,
    pub balance_icp: String,
    pub account_id: String,
}

/// Derive EVM address from ECDSA public key
pub fn derive_evm_address(public_key: [u8; 65]) -> [u8; 20] {
    // Remove the first byte (0x04 prefix for uncompressed key)
    let key_bytes = &public_key[1..];

    // Keccak256 hash
    let mut hasher = Keccak::v256();
    let mut hash = [0u8; 32];
    hasher.update(key_bytes);
    hasher.finalize(&mut hash);

    // Take last 20 bytes as address
    let mut address = [0u8; 20];
    address.copy_from_slice(&hash[12..]);
    address
}

/// Create and sign EVM transaction
pub async fn create_signed_transaction(
    args: &TransferEvmArgs,
    nonce: Option<u64>,
) -> Result<Vec<u8>, String> {
    // Build transaction
    let to_bytes = hex::decode(args.to.trim_start_matches("0x"))
        .map_err(|e| format!("Invalid to address: {}", e))?;

    // Convert Vec<u8> to [u8; 20]
    if to_bytes.len() != 20 {
        return Err(format!(
            "Invalid address length: expected 20 bytes, got {}",
            to_bytes.len()
        ));
    }

    let final_nonce = match nonce {
        Some(explicit_nonce) => {
            ic_cdk::println!("📊 Using provided nonce: {}", explicit_nonce);
            explicit_nonce
        }
        None => {
            ic_cdk::println!("📊 Fetching nonce from network...");
            get_transaction_count(args.wallet_id.clone(), args.chain_id)
                .await
                .map_err(|e| format!("Failed to get nonce: {}", e))?
        }
    };

    ic_cdk::println!("Using nonce: {}", final_nonce);

    let mut to_address = [0u8; 20];
    to_address.copy_from_slice(&to_bytes);

    let tx = LegacyTransaction {
        nonce: final_nonce.into(),
        gas_price: args.gas_price.into(),
        gas: args.gas_limit.into(),
        to: Some(to_address),
        value: args.value.into(),
        data: vec![],
        chain: args.chain_id,
    };

    // Get signing digest
    let digest = tx.hash();

    // Sign with threshold ECDSA
    let key_id = get_ecdsa_key_id_from_env("test");
    ic_cdk::println!("Using key: {}", key_id.name);
    let signature = sign_message(args.wallet_id.clone(), digest.to_vec(), key_id).await?;

    // Extract r, s from signature (first 64 bytes)
    if signature.len() < 65 {
        return Err("Invalid signature length".to_string());
    }

    let r = signature[0..32].to_vec();
    let s = signature[32..64].to_vec();
    let recovery_id = signature[64];

    // Calculate v for EIP-155
    let v = (recovery_id as u64) + 35 + args.chain_id * 2;

    // Create ECDSA signature object
    let ecdsa_sig = EcdsaSig { v, r, s };

    // Sign transaction
    let raw_signed_tx = tx.sign(&ecdsa_sig);

    Ok(raw_signed_tx)
}

/// Send transaction via EVM RPC canister
pub async fn send_transaction(raw_tx: Vec<u8>, chain_id: u64) -> Result<String, String> {
    let evm_canister_id =
        Principal::from_text(EVM_CANISTER_ID).map_err(|e| format!("Invalid canister ID: {}", e))?;

    let hex_raw = format!("0x{}", hex::encode(&raw_tx));

    // Use proper types
    let rpc_services = get_rpc_services(chain_id)?;

    let rpc_config = Some(RpcConfig {
        response_size_estimate: Some(1000),
        response_consensus: None,
    });

    ic_cdk::println!("Sending transaction: {}", hex_raw);

    // let (response,): (MultiSendRawTransactionResult,) = ic_cdk::api::call::call_with_payment(
    ic_cdk::api::call::call_with_payment(
        evm_canister_id,
        "eth_sendRawTransaction",
        (rpc_services, rpc_config, hex_raw),
        60_000_000_000,
    )
    .await
    .map_err(|e| format!("RPC call failed: {:?}", e))?;

    ic_cdk::println!("After sending transaction");
    Ok("Transaction sent successfully".to_string())

    // Handle response
    // match response {
    //     MultiSendRawTransactionResult::Consistent(result) => match result {
    //         SendRawTransactionResult::Ok(status) => match status {
    //             SendRawTransactionStatus::Ok(Some(tx_hash)) => {
    //                 println!("Transaction sentttttttttt");
    //                 println!("Transaction sent: {}", tx_hash);
    //                 Ok(tx_hash)
    //             }
    //             SendRawTransactionStatus::Ok(None) => {
    //                 Err("No transaction hash returned".to_string())
    //             }
    //             SendRawTransactionStatus::NonceTooLow => Err("Nonce too low".to_string()),
    //             SendRawTransactionStatus::NonceTooHigh => Err("Nonce too high".to_string()),
    //             SendRawTransactionStatus::InsufficientFunds => {
    //                 Err("Insufficient funds".to_string())
    //             }
    //         },
    //         SendRawTransactionResult::Err(rpc_error) => Err(format!("RPC error: {:?}", rpc_error)),
    //     },
    //     MultiSendRawTransactionResult::Inconsistent(results) => {
    //         // Try to find first successful result
    //         for (_service, result) in results {
    //             if let SendRawTransactionResult::Ok(SendRawTransactionStatus::Ok(Some(tx_hash))) =
    //                 result
    //             {
    //                 return Ok(tx_hash);
    //             }
    //         }
    //         Err("All providers failed".to_string())
    //     }
    // }
}

/// create and send EVM transaction
pub async fn transfer_evm(args: TransferEvmArgs, nonce: Option<u64>) -> Result<String, String> {
    // Create signed transaction
    let raw_tx = create_signed_transaction(&args, nonce).await?;

    // Send transaction
    send_transaction(raw_tx, args.chain_id).await
}

pub async fn get_transaction_count(wallet_id: String, chain_id: u64) -> Result<u64, String> {
    // Get EVM address from wallet_id
    let address = get_evm_address(wallet_id).await?;

    // Determine RPC services based on chain_id
    let rpc_services = get_rpc_services(chain_id)?;
    // let rpc_services = match chain_id {
    //     1 => RpcServices::EthMainnet(Some(vec![EthMainnetService::PublicNode])),
    //     11155111 => RpcServices::EthSepolia(Some(vec![EthSepoliaService::PublicNode])),
    //     42161 => RpcServices::ArbitrumOne(Some(vec![L2MainnetService::PublicNode])),
    //     421614 => RpcServices::Custom {
    //         chain_id: 421614,
    //         services: vec![RpcApi {
    //             url: "https://sepolia-rollup.arbitrum.io/rpc".to_string(),
    //             headers: None,
    //         }],
    //     },
    //     _ => return Err(format!("Unsupported chain_id: {}", chain_id)),
    // };

    // RPC config
    let rpc_config = Some(RpcConfig {
        response_size_estimate: Some(1000u64),
        response_consensus: None,
    });

    // Create GetTransactionCountArgs
    let args = GetTransactionCountArgs {
        address,
        block: BlockTag::Latest,
    };

    // EVM canister ID
    let evm_canister_id =
        Principal::from_text(EVM_CANISTER_ID).map_err(|e| format!("Invalid canister ID: {}", e))?;

    ic_cdk::println!("Getting transaction count for address: {}", args.address);
    ic_cdk::println!(
        "Getting transaction count for address 111: {}",
        args.address
    );

    // Call eth_getTransactionCount
    let (response,): (MultiGetTransactionCountResult,) = ic_cdk::api::call::call_with_payment(
        evm_canister_id,
        "eth_getTransactionCount",
        (rpc_services, rpc_config, args),
        10_000_000_000, // 10B cycles
    )
    .await
    .map_err(|e| format!("RPC call failed: {:?}", e))?;

    ic_cdk::println!("Received transaction count response");

    // Handle response
    match response {
        MultiGetTransactionCountResult::Consistent(result) => match result {
            GetTransactionCountResult::Ok(nonce) => {
                ic_cdk::println!("Transaction count: {}", nonce);
                let nonce_u64: u64 = nonce.0.try_into().map_err(|_| "Nonce too large for u64")?;
                Ok(nonce_u64)
            }
            GetTransactionCountResult::Err(rpc_error) => Err(format!("RPC error: {:?}", rpc_error)),
        },
        MultiGetTransactionCountResult::Inconsistent(results) => {
            Err(format!("Inconsistent results: {:?}", results))
        }
    }
}

/// Get EVM address for a wallet
pub async fn get_evm_address(wallet_id: String) -> Result<String, String> {
    let key_id = get_ecdsa_key_id_from_env("test");
    let public_key = get_public_key(wallet_id, key_id).await?;

    let address = derive_evm_address(public_key);
    Ok(format!("0x{}", hex::encode(address)))
}

fn get_token_contracts(chain_id: u64) -> Vec<(String, String)> {
    match chain_id {
        1 => vec![
            // Ethereum Mainnet
            (
                "USDC".to_string(),
                "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(),
            ),
            (
                "USDT".to_string(),
                "0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string(),
            ),
            (
                "WBTC".to_string(),
                "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599".to_string(),
            ),
        ],
        11155111 => vec![
            // Sepolia Testnet
            (
                "USDC".to_string(),
                "0xba1FCc7a596140e5feC52B3aB80a8F000C9Af104".to_string(),
            ), // USDC on Sepolia
            (
                "LINK".to_string(),
                "0x779877A7B0D9E8603169DdbD7836e478b4624789".to_string(),
            ), // LINK on Sepolia
            (
                "SepoliaMNT".to_string(),
                "0x65e37B558F64E2Be5768DB46DF22F93d85741A9E".to_string(),
            ), // DAI on Sepolia
        ],
        42161 => vec![
            // Arbitrum One
            (
                "USDC".to_string(),
                "0xaf88d065e77c8cC2239327C5EDb3A432268e5831".to_string(),
            ),
            (
                "ARB".to_string(),
                "0x912CE59144191C1204E64559FE8253a0e49E6548".to_string(),
            ),
        ],
        _ => vec![],
    }
}

pub async fn get_native_balance(wallet_id: String, chain_id: u64) -> Result<String, String> {
    let address = get_evm_address(wallet_id).await?;

    let json_request = json!({
        "jsonrpc": "2.0",
        "method": "eth_getBalance",
        "params": [address, "latest"],
        "id": 1
    })
    .to_string();

    ic_cdk::println!(
        "Getting native balance for {} on chain {}",
        address,
        chain_id
    );

    let evm_canister_id =
        Principal::from_text(EVM_CANISTER_ID).map_err(|e| format!("Invalid canister ID: {}", e))?;

    // Use raw call since we don't have proper bindings
    let (result,): (Result<String, String>,) = ic_cdk::api::call::call_with_payment(
        evm_canister_id,
        "request",
        (
            get_rpc_service(chain_id)?,
            json_request,
            1000u64, // max response bytes
        ),
        2_000_000_000, // cycles
    )
    .await
    .map_err(|e| format!("Call failed: {:?}", e))?;

    match result {
        Ok(response) => {
            let parsed: Value = serde_json::from_str(&response)
                .map_err(|e: serde_json::Error| format!("JSON parse error: {}", e))?;

            if let Some(error) = parsed.get("error") {
                return Err(format!("RPC error: {}", error));
            }

            let balance = parsed
                .get("result")
                .and_then(|r| r.as_str())
                .ok_or("No result in response")?
                .to_string();

            ic_cdk::println!("Native balance: {}", balance);
            Ok(balance)
        }
        Err(err) => Err(format!("RPC error: {}", err)),
    }
}

fn get_rpc_service(chain_id: u64) -> Result<RpcService, String> {
    match chain_id {
        1 => Ok(RpcService::EthMainnet(EthMainnetService::PublicNode)),
        11155111 => Ok(RpcService::EthSepolia(EthSepoliaService::PublicNode)),
        42161 => Ok(RpcService::ArbitrumOne(L2MainnetService::PublicNode)),
        421614 => Ok(RpcService::Custom(RpcApi {
            url: "https://sepolia-rollup.arbitrum.io/rpc".to_string(),
            headers: None,
        })),
        _ => Err(format!("Unsupported chain: {}", chain_id)),
    }
}

fn get_rpc_services(chain_id: u64) -> Result<RpcServices, String> {
    match chain_id {
        1 => Ok(RpcServices::EthMainnet(Some(vec![
            EthMainnetService::PublicNode,
        ]))),
        11155111 => Ok(RpcServices::EthSepolia(Some(vec![
            EthSepoliaService::PublicNode,
        ]))),
        42161 => Ok(RpcServices::ArbitrumOne(Some(vec![
            L2MainnetService::PublicNode,
        ]))),
        421614 => Ok(RpcServices::Custom {
            chain_id,
            services: vec![RpcApi {
                url: "https://sepolia-rollup.arbitrum.io/rpc".to_string(),
                headers: None,
            }],
        }),
        _ => Err(format!("Unsupported chain_id: {}", chain_id)),
    }
}

fn get_native_symbol(chain_id: u64) -> String {
    match chain_id {
        1 | 11155111 => "ETH".to_string(),
        42161 => "ETH".to_string(), // Arbitrum uses ETH
        _ => "ETH".to_string(),
    }
}

pub async fn get_erc20_balance(
    wallet_id: String,
    chain_id: u64,
    token_symbol: String,
    contract_address: String,
) -> Result<TokenBalance, String> {
    let address = get_evm_address(wallet_id).await?;

    // Create balanceOf call data
    let call_data = encode_balance_of(&address)?;

    let json_request = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [
            {
                "to": contract_address,
                "data": call_data
            },
            "latest"
        ],
        "id": 1
    })
    .to_string();

    ic_cdk::println!("Getting {} balance for {}", token_symbol, address);

    let evm_canister_id =
        Principal::from_text(EVM_CANISTER_ID).map_err(|e| format!("Invalid canister ID: {}", e))?;

    let (result,): (Result<String, String>,) = ic_cdk::api::call::call_with_payment128(
        evm_canister_id,
        "request",
        (get_rpc_service(chain_id)?, json_request, 1000u64),
        2_000_000_000u128,
    )
    .await
    .map_err(|e| format!("Call failed: {:?}", e))?;

    match result {
        Ok(response) => {
            let parsed: Value =
                serde_json::from_str(&response).map_err(|e| format!("JSON parse error: {}", e))?;

            if let Some(error) = parsed.get("error") {
                return Ok(TokenBalance {
                    symbol: token_symbol,
                    contract_address: contract_address,
                    balance: "0".to_string(),
                    success: false,
                    error: Some(format!("RPC error: {}", error)),
                });
            }

            let result_hex = parsed
                .get("result")
                .and_then(|r| r.as_str())
                .unwrap_or("0x0");

            ic_cdk::println!("{} balance (hex): {}", token_symbol, result_hex);

            Ok(TokenBalance {
                symbol: token_symbol,
                contract_address: contract_address,
                balance: result_hex.to_string(),
                success: true,
                error: None,
            })
        }
        Err(err) => Ok(TokenBalance {
            symbol: token_symbol,
            contract_address: contract_address,
            balance: "0".to_string(),
            success: false,
            error: Some(err),
        }),
    }
}

fn encode_balance_of(address: &str) -> Result<String, String> {
    // balanceOf(address) function signature: 0x70a08231
    let function_selector = "70a08231";

    // Clean and pad address to 32 bytes
    let clean_address = address.trim_start_matches("0x");
    if clean_address.len() != 40 {
        return Err(format!("Invalid address length: {}", clean_address.len()));
    }

    let padded_address = format!("{:0>64}", clean_address);
    Ok(format!("0x{}{}", function_selector, padded_address))
}

pub async fn get_wallet_portfolio(
    wallet_id: String,
    chain_id: u64,
) -> Result<PortfolioBalance, String> {
    let wallet_address = get_evm_address(wallet_id.clone()).await?;

    ic_cdk::println!(
        "Getting portfolio for {} on chain {}",
        wallet_address,
        chain_id
    );

    let icp_balance = get_icp_balance(wallet_id.clone())
        .await
        .unwrap_or_else(|e| {
            ic_cdk::println!("Failed to get ICP balance: {}", e);
            ICPBalance {
                balance_e8s: 0,
                balance_icp: "0.00000000".to_string(),
                account_id: "".to_string(),
            }
        });

    // Get native balance
    let native_balance = get_native_balance(wallet_id.clone(), chain_id)
        .await
        .unwrap_or_else(|e| {
            ic_cdk::println!("Failed to get native balance: {}", e);
            "0x0".to_string()
        });

    // Get token contracts for this chain
    // let token_contracts = get_token_contracts(chain_id);
    let token_contracts = vec![]; // --- IGNORE ---
    let mut token_balances = Vec::new();

    // Get each token balance
    for (symbol, contract_address) in token_contracts {
        let token_balance =
            get_erc20_balance(wallet_id.clone(), chain_id, symbol, contract_address)
                .await
                .unwrap_or_else(|e| TokenBalance {
                    symbol: "ERROR".to_string(),
                    contract_address: "".to_string(),
                    balance: "0".to_string(),
                    success: false,
                    error: Some(e),
                });

        token_balances.push(token_balance);
    }

    Ok(PortfolioBalance {
        icp_balance,
        native_balance,
        native_symbol: get_native_symbol(chain_id),
        token_balances,
        wallet_address,
        chain_id,
    })
}

fn principal_to_account_id(principal: &Principal, subaccount: Option<[u8; 32]>) -> [u8; 32] {
    let mut data: Vec<u8> = vec![0x0A];
    data.extend(b"account-id");
    data.extend(principal.as_slice());
    data.extend(subaccount.unwrap_or([0u8; 32]));

    // SHA224 hash
    let hash = Sha224::digest(&data);

    // CRC32 checksum
    let mut crc32 = Crc32Hasher::new();
    crc32.update(&hash);
    let checksum = crc32.finalize().to_be_bytes();

    // Combine checksum and hash
    let mut account_id = [0u8; 32];
    account_id[..4].copy_from_slice(&checksum);
    account_id[4..].copy_from_slice(&hash);

    account_id
}

pub async fn get_icp_balance(wallet_id: String) -> Result<ICPBalance, String> {
    ic_cdk::println!("Getting ICP balance for wallet: {}", wallet_id);

    // Get canister principal
    let canister_principal = ic_cdk::id();

    // Generate account ID
    let account_id = principal_to_account_id(&canister_principal, None);

    ic_cdk::println!("Canister Principal: {}", canister_principal);
    ic_cdk::println!("Account ID: {}", hex::encode(&account_id));

    // Use local ledger canister ID
    let ledger_canister_id = Principal::from_text(LEDGER_CANISTER_ID)
        .map_err(|e| format!("Invalid ledger canister ID: {}", e))?;

    let args = AccountBalanceArgs {
        account: account_id.to_vec(),
    };

    // Call ICP Ledger
    let (tokens,): (Tokens,) =
        ic_cdk::api::call::call(ledger_canister_id, "account_balance", (args,))
            .await
            .map_err(|e| format!("Call failed: {:?}", e))?;

    let balance_e8s = tokens.e8s;
    let balance_icp: f64 = (balance_e8s as f64) / 100_000_000.0;

    ic_cdk::println!("ICP Balance: {} e8s ({} ICP)", balance_e8s, balance_icp);

    Ok(ICPBalance {
        balance_e8s,
        balance_icp: format!("{:.8}", balance_icp),
        account_id: hex::encode(&account_id),
    })
}
