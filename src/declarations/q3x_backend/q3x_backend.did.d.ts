import type { Principal } from '@dfinity/principal';
import type { ActorMethod } from '@dfinity/agent';
import type { IDL } from '@dfinity/candid';

export interface BatchTransaction {
  'id' : string,
  'description' : string,
  'created_at' : bigint,
  'created_by' : Principal,
  'transactions' : Array<TransactionType>,
}
export interface ICPBalance {
  'account_id' : string,
  'balance_e8s' : bigint,
  'balance_icp' : string,
}
export interface PortfolioBalance {
  'native_symbol' : string,
  'wallet_address' : string,
  'chain_id' : bigint,
  'token_balances' : Array<TokenBalance>,
  'icp_balance' : ICPBalance,
  'native_balance' : string,
}
export type Result = { 'Ok' : null } |
  { 'Err' : string };
export type Result_1 = { 'Ok' : string } |
  { 'Err' : string };
export type Result_2 = { 'Ok' : number } |
  { 'Err' : string };
export type Result_3 = { 'Ok' : ICPBalance } |
  { 'Err' : string };
export type Result_4 = { 'Ok' : Array<string> } |
  { 'Err' : string };
export type Result_5 = { 'Ok' : Array<[string, Array<string>]> } |
  { 'Err' : string };
export type Result_6 = { 'Ok' : bigint } |
  { 'Err' : string };
export type Result_7 = { 'Ok' : PortfolioBalance } |
  { 'Err' : string };
export type Result_8 = { 'Ok' : boolean } |
  { 'Err' : string };
export interface TokenBalance {
  'balance' : string,
  'error' : [] | [string],
  'success' : boolean,
  'contract_address' : string,
  'symbol' : string,
}
export type TransactionType = {
    'IcpTransfer' : {
      'to_principal' : Principal,
      'to_subaccount' : [] | [Uint8Array | number[]],
      'memo' : [] | [bigint],
      'amount' : bigint,
    }
  } |
  {
    'EvmTransfer' : {
      'to' : string,
      'value' : bigint,
      'chain_id' : bigint,
      'gas_limit' : bigint,
      'gas_price' : bigint,
    }
  };
export interface TransferEvmArgs {
  'to' : string,
  'value' : bigint,
  'chain_id' : bigint,
  'gas_limit' : bigint,
  'wallet_id' : string,
  'gas_price' : bigint,
}
export interface Wallet {
  'threshold' : number,
  'metadata' : Array<[Uint8Array | number[], string]>,
  'signers' : Array<string>,
  'message_queue' : Array<[Uint8Array | number[], Array<string>]>,
}
export interface _SERVICE {
  /**
   * Add metadata to a message in the wallet.
   * 
   * * `message` - The message as a `Vec<u8>`.
   * * `metadata` - The metadata as a `String`.
   * * `caller` - The `Principal` of the caller.
   * 
   * Returns `Result<(), String>` indicating success or the type of failure.
   */
  'add_metadata' : ActorMethod<[string, string, string], Result>,
  /**
   * Proposes adding a new signer to the wallet.
   * 
   * # Arguments
   * 
   * * `wallet_id` - The wallet's unique identifier.
   * * `new_signer` - The Principal of the new signer to add.
   * 
   * # Returns
   * 
   * * `Result<(), String>` - Result indicating success or an error message.
   */
  'add_signer' : ActorMethod<[string, Principal], Result_1>,
  /**
   * Approves a message for signing in the wallet.
   * 
   * # Arguments
   * 
   * * `wallet_id` - The wallet's unique identifier.
   * * `msg` - The message to be approved, in hexadecimal format.
   * 
   * # Returns
   * 
   * * `Result<u8, String>` - The number of signatures or an error message.
   */
  'approve' : ActorMethod<[string, string], Result_2>,
  /**
   * Checks if a message can be signed by the wallet.
   * 
   * # Arguments
   * 
   * * `wallet_id` - The wallet's unique identifier.
   * * `msg` - The message to be checked, in hexadecimal format.
   * 
   * # Returns
   * 
   * * `bool` - True if the message can be signed, otherwise false.
   */
  'can_sign' : ActorMethod<[string, string], boolean>,
  /**
   * Creates a new wallet.
   * 
   * # Arguments
   * 
   * * `wallet_id` - Unique identifier for the wallet as a String.
   * * `signers` - A list of Principals representing the signers of the wallet.
   * * `threshold` - The threshold number of signers required for a transaction.
   * 
   * # Returns
   * 
   * * `Result<(), String>` - Result indicating success or an error message.
   */
  'create_wallet' : ActorMethod<[string, Array<Principal>, number], Result>,
  'get_evm_address' : ActorMethod<[string], Result_1>,
  'get_icp_balance' : ActorMethod<[string], Result_3>,
  /**
   * Retrieves all messages that can be signed for a given wallet.
   * 
   * # Arguments
   * 
   * * `wallet_id` - The wallet's unique identifier.
   * 
   * # Returns
   * 
   * * `Vec<Vec<u8>>` - A list of messages that can be signed.
   */
  'get_messages_to_sign' : ActorMethod<[string], Result_4>,
  /**
   * Retrieves all messages that have been proposed along with their signers for a given wallet.
   * 
   * # Arguments
   * 
   * * `wallet_id` - The wallet's unique identifier.
   * 
   * # Returns
   * 
   * * `Vec<(Vec<u8>, Vec<String>)>` - A list of tuples containing messages and their signers (hex-string).
   */
  'get_messages_with_signers' : ActorMethod<[string], Result_5>,
  /**
   * Get the metadata associated with a message in the wallet.
   * 
   * * `message` - The message as a `Vec<u8>`.
   * 
   * Returns `Option<&String>` containing the metadata if it exists.
   */
  'get_metadata' : ActorMethod<[string, string], Result_1>,
  /**
   * Retrieves all messages that have been proposed for a given wallet.
   * 
   * # Arguments
   * 
   * * `wallet_id` - The wallet's unique identifier.
   * 
   * # Returns
   * 
   * * `Vec<Vec<u8>>` - A list of messages that have been proposed.
   */
  'get_proposed_messages' : ActorMethod<[string], Result_4>,
  'get_transaction_count' : ActorMethod<[string, bigint], Result_6>,
  /**
   * Retrieves a wallet by its ID.
   * 
   * # Arguments
   * 
   * * `wallet_id` - The unique identifier for the wallet as a String.
   * 
   * # Returns
   * 
   * * `Option<Wallet>` - The wallet if found, otherwise None.
   */
  'get_wallet' : ActorMethod<[string], [] | [Wallet]>,
  'get_wallet_portfolio' : ActorMethod<[string, bigint], Result_7>,
  /**
   * Retrieves all wallets associated with a given principal.
   * 
   * # Arguments
   * 
   * * `principal` - The principal to retrieve wallets for.
   * 
   * # Returns
   * 
   * * `HashSet<String>` - A list of wallet IDs associated with the principal.
   */
  'get_wallets_for_principal' : ActorMethod<[Principal], Array<string>>,
  /**
   * Proposes a message to be signed by the wallet.
   * 
   * # Arguments
   * 
   * * `wallet_id` - The wallet's unique identifier.
   * * `msg` - The message to be proposed, in hexadecimal format.
   * 
   * # Returns
   * 
   * * `Result<(), String>` - Result indicating success or an error message.
   */
  'propose' : ActorMethod<[string, string], Result>,
  'propose_batch_transaction' : ActorMethod<
    [string, BatchTransaction],
    Result_1
  >,
  /**
   * Proposes a message and adds metadata in one call.
   * 
   * # Arguments
   * 
   * * `wallet_id` - The wallet's unique identifier.
   * * `msg` - The message to be proposed, in hexadecimal format.
   * * `metadata` - The metadata to be added to the message.
   * 
   * # Returns
   * 
   * * `Result<(), String>` - Result indicating success or an error message.
   */
  'propose_with_metadata' : ActorMethod<[string, string, string], Result>,
  /**
   * Proposes removing a signer from the wallet.
   * 
   * # Arguments
   * 
   * * `wallet_id` - The wallet's unique identifier.
   * * `signer_to_remove` - The Principal of the signer to remove.
   * 
   * # Returns
   * 
   * * `Result<(), String>` - Result indicating success or an error message.
   */
  'remove_signer' : ActorMethod<[string, Principal], Result_1>,
  /**
   * Proposes setting a new threshold for the wallet.
   * 
   * # Arguments
   * 
   * * `wallet_id` - The wallet's unique identifier.
   * * `new_threshold` - The new threshold value to set.
   * 
   * # Returns
   * 
   * * `Result<String, String>` - Result indicating success or an error message.
   */
  'set_threshold' : ActorMethod<[string, number], Result_1>,
  /**
   * Signs a message using the wallet.
   * 
   * # Arguments
   * 
   * * `wallet_id` - The wallet's unique identifier.
   * * `msg` - The message to be signed, in hexadecimal format.
   * 
   * # Returns
   * 
   * * `Result<String, String>` - The signature in hexadecimal format or an error message.
   */
  'sign' : ActorMethod<[string, string], Result_1>,
  'transfer' : ActorMethod<[string, bigint, Principal], Result_1>,
  'transfer_evm' : ActorMethod<[TransferEvmArgs], Result_1>,
  /**
   * Verifies a signature for a given message and wallet.
   * 
   * # Arguments
   * 
   * * `wallet_id` - The wallet's unique identifier.
   * * `message` - The message associated with the signature, in hexadecimal format.
   * * `signature` - The signature to be verified, in hexadecimal format.
   * 
   * # Returns
   * 
   * * `Result<bool, String>` - True if the signature is valid, otherwise an error message.
   */
  'verify_signature' : ActorMethod<[string, string, string], Result_8>,
}
export declare const idlFactory: IDL.InterfaceFactory;
export declare const init: (args: { IDL: typeof IDL }) => IDL.Type[];
