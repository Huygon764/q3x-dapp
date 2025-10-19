use candid::{CandidType, Principal};
use ic_ledger_types::{
    AccountIdentifier, BlockIndex, Memo, Subaccount, Tokens, DEFAULT_SUBACCOUNT,
    MAINNET_LEDGER_CANISTER_ID,
};
use serde::{Deserialize, Serialize};

use crate::{
    chain_fusion::{get_transaction_count, transfer_evm, TransferEvmArgs},
    propose,
    wallet::{self, MultiSignatureWallet, TransferArgs},
    WALLETS, WALLET_NOT_FOUND_ERROR,
};

#[derive(CandidType, Deserialize, Serialize, Debug, Clone)]
pub enum TransactionType {
    IcpTransfer {
        amount: u64,
        to_principal: Principal,
        to_subaccount: Option<Subaccount>,
        memo: Option<u64>,
    },
    EvmTransfer {
        to: String,
        value: u128,
        chain_id: u64,
        gas_price: u128,
        gas_limit: u64,
    },
}

#[derive(CandidType, Deserialize, Serialize, Debug, Clone)]
pub struct BatchTransaction {
    pub id: String,
    pub transactions: Vec<TransactionType>,
    pub description: String,
    pub created_at: u64,
    pub created_by: Principal,
}

pub async fn propose_batch(wallet_id: String, batch: BatchTransaction) -> Result<String, String> {
    ic_cdk::println!("📦 Creating batch transaction proposal");
    ic_cdk::println!("Batch ID: {}", batch.id);
    ic_cdk::println!("Description: {}", batch.description);
    ic_cdk::println!("Transactions: {}", batch.transactions.len());
    ic_cdk::println!("Created by: {}", batch.created_by);

    // Validate batch
    if batch.transactions.is_empty() {
        return Err("Batch cannot be empty".to_string());
    }

    if batch.transactions.len() > 50 {
        return Err("Batch too large (max 50 transactions)".to_string());
    }

    // Serialize batch to bytes
    let message_bytes =
        candid::encode_one(&batch).map_err(|e| format!("Serialization failed: {}", e))?;

    let message_hex = hex::encode(&message_bytes);

    let special_message = hex::encode(format!("BATCH::{}::{}", batch.id, message_hex));

    ic_cdk::println!("🔐 Proposing batch to special_message: {}", special_message);

    ic_cdk::println!(
        "📝 Serialized batch message length: {} bytes",
        special_message.len()
    );

    // Use existing propose mechanism
    let _ = propose(wallet_id, special_message.clone()).await?;
    // let _ = propose(wallet_id, message_hex).await?;
    Ok(("Proposal for batch transaction created successfully").to_string())
}

pub async fn execute_batch(wallet_id: String, batch: BatchTransaction) -> Result<String, String> {
    ic_cdk::println!(
        "🚀 Executing batch: {} ({} transactions)",
        batch.id,
        batch.transactions.len()
    );
    ic_cdk::println!("📋 Description: {}", batch.description);

    // Get starting nonce for EVM transactions
    let mut current_nonce: Option<u64> = None;
    let mut nonce_offset = 0;

    // Find first EVM transaction and get current nonce
    for transaction in &batch.transactions {
        if let TransactionType::EvmTransfer { chain_id, .. } = transaction {
            ic_cdk::println!("📊 Getting transaction count for chain: {}", chain_id);
            current_nonce = Some(
                get_transaction_count(wallet_id.clone(), *chain_id)
                    .await
                    .map_err(|e| format!("Failed to get nonce: {}", e))?,
            );
            ic_cdk::println!("📊 Starting nonce: {:?}", current_nonce);
            break;
        }
    }

    let mut results = Vec::new();
    let mut success_count = 0;
    let mut failure_count = 0;

    // Execute each transaction independently
    for (index, transaction) in batch.transactions.iter().enumerate() {
        ic_cdk::println!(
            "📤 Processing transaction {}/{}",
            index + 1,
            batch.transactions.len()
        );

        let result = match transaction {
            TransactionType::IcpTransfer {
                amount,
                to_principal,
                to_subaccount,
                memo,
            } => {
                ic_cdk::println!("💎 ICP Transfer: {} e8s to {}", amount, to_principal);
                let args_icp = TransferArgs {
                    amount: Tokens::from_e8s(*amount),
                    to_principal: *to_principal,
                    to_subaccount: *to_subaccount,
                };
                transfer_icp(args_icp)
                    .await
                    .map(|block_index| block_index.to_string())
            }
            TransactionType::EvmTransfer {
                to,
                value,
                chain_id,
                gas_price,
                gas_limit,
            } => {
                ic_cdk::println!(
                    "⚡ EVM Transfer: {} Wei to {} on chain {}",
                    value,
                    to,
                    chain_id
                );
                let nonce = current_nonce.map(|base| base + nonce_offset);
                let args_evm = TransferEvmArgs {
                    wallet_id: wallet_id.clone(),
                    to: to.clone(),
                    value: *value,
                    chain_id: *chain_id,
                    gas_price: *gas_price,
                    gas_limit: *gas_limit,
                };
                nonce_offset += 1;
                transfer_evm(args_evm, nonce).await
            }
        };

        match result {
            Ok(tx_result) => {
                ic_cdk::println!("✅ Transaction {} success: {}", index + 1, tx_result);
                success_count += 1;
                results.push(format!("TX{}: SUCCESS - {}", index + 1, tx_result));
            }
            Err(error) => {
                ic_cdk::println!("❌ Transaction {} failed: {}", index + 1, error);
                failure_count += 1;
                results.push(format!("TX{}: FAILED - {}", index + 1, error));
                // Continue to next transaction (no rollback)
            }
        }
    }

    let summary = format!(
        "Batch {} completed: {}/{} successful, {}/{} failed",
        batch.id,
        success_count,
        batch.transactions.len(),
        failure_count,
        batch.transactions.len()
    );

    ic_cdk::println!("📊 Final Summary: {}", summary);

    // Always return Ok with summary (even if some transactions failed)
    Ok(format!("{}\nDetails:\n{}", summary, results.join("\n")))
}

// transfer icp token
pub async fn transfer_icp(args: TransferArgs) -> Result<BlockIndex, String> {
    ic_cdk::println!(
        "Transferring {} tokens to principal {} subaccount {:?}",
        &args.amount,
        &args.to_principal,
        &args.to_subaccount
    );
    let to_subaccount = args.to_subaccount.unwrap_or(DEFAULT_SUBACCOUNT);
    let transfer_args = ic_ledger_types::TransferArgs {
        memo: Memo(0),
        amount: args.amount,
        fee: Tokens::from_e8s(10_000),
        // The subaccount of the account identifier that will be used to withdraw tokens and send them
        // to another account identifier. If set to None then the default subaccount will be used.
        // See the [Ledger doc](https://internetcomputer.org/docs/current/developer-docs/integrations/ledger/#accounts).
        from_subaccount: None,
        to: AccountIdentifier::new(&args.to_principal, &to_subaccount),
        created_at_time: None,
    };
    ic_ledger_types::transfer(MAINNET_LEDGER_CANISTER_ID, &transfer_args)
        .await
        .map_err(|e| format!("failed to call ledger: {e:?}"))?
        .map_err(|e| format!("ledger transfer error {e:?}"))
}
