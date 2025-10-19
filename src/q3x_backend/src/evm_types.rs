// This is an experimental feature to generate Rust binding from Candid.
// You may want to manually adjust some of the types.
#![allow(dead_code, unused_imports)]
use candid::{self, CandidType, Deserialize, Principal};
use ic_cdk::api::call::CallResult as Result;

pub type Regex = String;
#[derive(CandidType, Deserialize, Debug)]
pub enum LogFilter { ShowAll, HideAll, ShowPattern(Regex), HidePattern(Regex) }
#[derive(CandidType, Deserialize, Debug)]
pub struct RegexSubstitution { pub pattern: Regex, pub replacement: String }
/// Override resolved provider.
/// Useful for testing with a local Ethereum developer environment such as foundry.
#[derive(CandidType, Deserialize, Debug)]
pub struct OverrideProvider {
  #[serde(rename="overrideUrl")]
  pub override_url: Option<RegexSubstitution>,
}
#[derive(CandidType, Deserialize, Debug)]
pub struct InstallArgs {
  #[serde(rename="logFilter")]
  pub log_filter: Option<LogFilter>,
  pub demo: Option<bool>,
  #[serde(rename="manageApiKeys")]
  pub manage_api_keys: Option<Vec<Principal>>,
  #[serde(rename="overrideProvider")]
  pub override_provider: Option<OverrideProvider>,
  #[serde(rename="nodesInSubnet")]
  pub nodes_in_subnet: Option<u32>,
}
#[derive(CandidType, Deserialize, Debug)]
pub enum EthSepoliaService { Alchemy, BlockPi, PublicNode, Ankr, Sepolia }
#[derive(CandidType, Deserialize, Debug)]
pub enum L2MainnetService { Alchemy, Llama, BlockPi, PublicNode, Ankr }
pub type ChainId = u64;
#[derive(CandidType, Deserialize, Debug)]
pub struct HttpHeader { pub value: String, pub name: String }
#[derive(CandidType, Deserialize, Debug)]
pub struct RpcApi { pub url: String, pub headers: Option<Vec<HttpHeader>> }
#[derive(CandidType, Deserialize, Debug)]
pub enum EthMainnetService {
  Alchemy,
  Llama,
  BlockPi,
  Cloudflare,
  PublicNode,
  Ankr,
}
#[derive(CandidType, Deserialize, Debug)]
pub enum RpcServices {
  EthSepolia(Option<Vec<EthSepoliaService>>),
  BaseMainnet(Option<Vec<L2MainnetService>>),
  Custom{ #[serde(rename="chainId")] chain_id: ChainId, services: Vec<RpcApi> },
  OptimismMainnet(Option<Vec<L2MainnetService>>),
  ArbitrumOne(Option<Vec<L2MainnetService>>),
  EthMainnet(Option<Vec<EthMainnetService>>),
}
#[derive(CandidType, Deserialize, Debug)]
pub enum ConsensusStrategy {
  Equality,
  Threshold{
    /// Minimum number of providers that must return the same (non-error) result.
    min: u8,
    /// Total number of providers to be queried. Can be omitted, if that number can be inferred (e.g., providers are specified in the request).
    total: Option<u8>,
  },
}
#[derive(CandidType, Deserialize, Debug)]
pub struct RpcConfig {
  #[serde(rename="responseConsensus")]
  pub response_consensus: Option<ConsensusStrategy>,
  #[serde(rename="responseSizeEstimate")]
  pub response_size_estimate: Option<u64>,
}
#[derive(CandidType, Deserialize, Debug)]
pub struct AccessListEntry {
  #[serde(rename="storageKeys")]
  pub storage_keys: Vec<String>,
  pub address: String,
}
#[derive(CandidType, Deserialize, Debug)]
pub struct TransactionRequest {
  pub to: Option<String>,
  pub gas: Option<candid::Nat>,
  #[serde(rename="maxFeePerGas")]
  pub max_fee_per_gas: Option<candid::Nat>,
  #[serde(rename="gasPrice")]
  pub gas_price: Option<candid::Nat>,
  pub value: Option<candid::Nat>,
  #[serde(rename="maxFeePerBlobGas")]
  pub max_fee_per_blob_gas: Option<candid::Nat>,
  pub from: Option<String>,
  pub r#type: Option<String>,
  #[serde(rename="accessList")]
  pub access_list: Option<Vec<AccessListEntry>>,
  pub nonce: Option<candid::Nat>,
  #[serde(rename="maxPriorityFeePerGas")]
  pub max_priority_fee_per_gas: Option<candid::Nat>,
  pub blobs: Option<Vec<String>>,
  pub input: Option<String>,
  #[serde(rename="chainId")]
  pub chain_id: Option<candid::Nat>,
  #[serde(rename="blobVersionedHashes")]
  pub blob_versioned_hashes: Option<Vec<String>>,
}
#[derive(CandidType, Deserialize, Debug)]
pub enum BlockTag {
  Earliest,
  Safe,
  Finalized,
  Latest,
  Number(candid::Nat),
  Pending,
}
#[derive(CandidType, Deserialize, Debug)]
pub struct CallArgs {
  pub transaction: TransactionRequest,
  pub block: Option<BlockTag>,
}
#[derive(CandidType, Deserialize, Debug)]
pub struct JsonRpcError { pub code: i64, pub message: String }
#[derive(CandidType, Deserialize, Debug)]
pub enum ProviderError {
  TooFewCycles{ expected: candid::Nat, received: candid::Nat },
  InvalidRpcConfig(String),
  MissingRequiredProvider,
  ProviderNotFound,
  NoPermission,
}
#[derive(CandidType, Deserialize, Debug)]
pub enum ValidationError { Custom(String), InvalidHex(String) }
#[derive(CandidType, Deserialize, Debug)]
pub enum RejectionCode {
  NoError,
  CanisterError,
  SysTransient,
  DestinationInvalid,
  Unknown,
  SysFatal,
  CanisterReject,
}
#[derive(CandidType, Deserialize, Debug)]
pub enum HttpOutcallError {
  IcError{ code: RejectionCode, message: String },
  InvalidHttpJsonRpcResponse{
    status: u16,
    body: String,
    #[serde(rename="parsingError")]
    parsing_error: Option<String>,
  },
}
#[derive(CandidType, Deserialize, Debug)]
pub enum RpcError {
  JsonRpcError(JsonRpcError),
  ProviderError(ProviderError),
  ValidationError(ValidationError),
  HttpOutcallError(HttpOutcallError),
}
pub type CallResult = std::result::Result<String, RpcError>;
pub type ProviderId = u64;
#[derive(CandidType, Deserialize, Debug)]
pub enum RpcService {
  EthSepolia(EthSepoliaService),
  BaseMainnet(L2MainnetService),
  Custom(RpcApi),
  OptimismMainnet(L2MainnetService),
  ArbitrumOne(L2MainnetService),
  EthMainnet(EthMainnetService),
  Provider(ProviderId),
}
#[derive(CandidType, Deserialize, Debug)]
pub enum MultiCallResult {
  Consistent(CallResult),
  Inconsistent(Vec<(RpcService,CallResult,)>),
}
#[derive(CandidType, Deserialize, Debug)]
pub struct FeeHistoryArgs {
  #[serde(rename="blockCount")]
  pub block_count: candid::Nat,
  #[serde(rename="newestBlock")]
  pub newest_block: BlockTag,
  #[serde(rename="rewardPercentiles")]
  pub reward_percentiles: Option<serde_bytes::ByteBuf>,
}
#[derive(CandidType, Deserialize, Debug)]
pub struct FeeHistory {
  pub reward: Vec<Vec<candid::Nat>>,
  #[serde(rename="gasUsedRatio")]
  pub gas_used_ratio: Vec<f64>,
  #[serde(rename="oldestBlock")]
  pub oldest_block: candid::Nat,
  #[serde(rename="baseFeePerGas")]
  pub base_fee_per_gas: Vec<candid::Nat>,
}
pub type FeeHistoryResult = std::result::Result<FeeHistory, RpcError>;
#[derive(CandidType, Deserialize, Debug)]
pub enum MultiFeeHistoryResult {
  Consistent(FeeHistoryResult),
  Inconsistent(Vec<(RpcService,FeeHistoryResult,)>),
}
#[derive(CandidType, Deserialize, Debug)]
pub struct Block {
  pub miner: String,
  #[serde(rename="totalDifficulty")]
  pub total_difficulty: Option<candid::Nat>,
  #[serde(rename="receiptsRoot")]
  pub receipts_root: String,
  #[serde(rename="stateRoot")]
  pub state_root: String,
  pub hash: String,
  pub difficulty: Option<candid::Nat>,
  pub size: candid::Nat,
  pub uncles: Vec<String>,
  #[serde(rename="baseFeePerGas")]
  pub base_fee_per_gas: Option<candid::Nat>,
  #[serde(rename="extraData")]
  pub extra_data: String,
  #[serde(rename="transactionsRoot")]
  pub transactions_root: Option<String>,
  #[serde(rename="sha3Uncles")]
  pub sha_3_uncles: String,
  pub nonce: candid::Nat,
  pub number: candid::Nat,
  pub timestamp: candid::Nat,
  pub transactions: Vec<String>,
  #[serde(rename="gasLimit")]
  pub gas_limit: candid::Nat,
  #[serde(rename="logsBloom")]
  pub logs_bloom: String,
  #[serde(rename="parentHash")]
  pub parent_hash: String,
  #[serde(rename="gasUsed")]
  pub gas_used: candid::Nat,
  #[serde(rename="mixHash")]
  pub mix_hash: String,
}
pub type GetBlockByNumberResult = std::result::Result<Block, RpcError>;
#[derive(CandidType, Deserialize, Debug)]
pub enum MultiGetBlockByNumberResult {
  Consistent(GetBlockByNumberResult),
  Inconsistent(Vec<(RpcService,GetBlockByNumberResult,)>),
}
#[derive(CandidType, Deserialize, Debug)]
pub struct GetLogsRpcConfig {
  #[serde(rename="responseConsensus")]
  pub response_consensus: Option<ConsensusStrategy>,
  /// Most providers limit the block range that can be queried with `eth_getLogs`.
  /// By default, the chosen value should work for all supported JSON-RPC providers.
  /// This value could be changed if the providers are explicitly specified and they do support querying a larger block range.
  /// DEFAULT: 500
  #[serde(rename="maxBlockRange")]
  pub max_block_range: Option<u32>,
  #[serde(rename="responseSizeEstimate")]
  pub response_size_estimate: Option<u64>,
}
/// Each topic is a `vec text` of topic data composed with the "or" operator.
/// See https://ethereum.org/en/developers/docs/apis/json-rpc/#eth_getlogs
pub type Topic = Vec<String>;
#[derive(CandidType, Deserialize, Debug)]
pub struct GetLogsArgs {
  #[serde(rename="fromBlock")]
  pub from_block: Option<BlockTag>,
  #[serde(rename="toBlock")]
  pub to_block: Option<BlockTag>,
  pub addresses: Vec<String>,
  pub topics: Option<Vec<Topic>>,
}
#[derive(CandidType, Deserialize, Debug)]
pub struct LogEntry {
  #[serde(rename="transactionHash")]
  pub transaction_hash: Option<String>,
  #[serde(rename="blockNumber")]
  pub block_number: Option<candid::Nat>,
  pub data: String,
  #[serde(rename="blockHash")]
  pub block_hash: Option<String>,
  #[serde(rename="transactionIndex")]
  pub transaction_index: Option<candid::Nat>,
  pub topics: Vec<String>,
  pub address: String,
  #[serde(rename="logIndex")]
  pub log_index: Option<candid::Nat>,
  pub removed: bool,
}
pub type GetLogsResult = std::result::Result<Vec<LogEntry>, RpcError>;
#[derive(CandidType, Deserialize, Debug)]
pub enum MultiGetLogsResult {
  Consistent(GetLogsResult),
  Inconsistent(Vec<(RpcService,GetLogsResult,)>),
}
#[derive(CandidType, Deserialize, Debug)]
pub struct GetTransactionCountArgs { pub address: String, pub block: BlockTag }
pub type GetTransactionCountResult = std::result::Result<candid::Nat, RpcError>;
#[derive(CandidType, Deserialize, Debug)]
pub enum MultiGetTransactionCountResult {
  Consistent(GetTransactionCountResult),
  Inconsistent(Vec<(RpcService,GetTransactionCountResult,)>),
}
#[derive(CandidType, Deserialize, Debug)]
pub struct TransactionReceipt {
  pub to: Option<String>,
  pub status: Option<candid::Nat>,
  #[serde(rename="transactionHash")]
  pub transaction_hash: String,
  #[serde(rename="blockNumber")]
  pub block_number: candid::Nat,
  pub from: String,
  pub logs: Vec<LogEntry>,
  #[serde(rename="blockHash")]
  pub block_hash: String,
  pub r#type: String,
  #[serde(rename="transactionIndex")]
  pub transaction_index: candid::Nat,
  #[serde(rename="effectiveGasPrice")]
  pub effective_gas_price: candid::Nat,
  #[serde(rename="logsBloom")]
  pub logs_bloom: String,
  #[serde(rename="contractAddress")]
  pub contract_address: Option<String>,
  #[serde(rename="gasUsed")]
  pub gas_used: candid::Nat,
}
pub type GetTransactionReceiptResult = std::result::Result<
  Option<TransactionReceipt>, RpcError
>;
#[derive(CandidType, Deserialize, Debug)]
pub enum MultiGetTransactionReceiptResult {
  Consistent(GetTransactionReceiptResult),
  Inconsistent(Vec<(RpcService,GetTransactionReceiptResult,)>),
}
#[derive(CandidType, Deserialize, Debug)]
pub enum SendRawTransactionStatus {
  Ok(Option<String>),
  NonceTooLow,
  NonceTooHigh,
  InsufficientFunds,
}
pub type SendRawTransactionResult = std::result::Result<
  SendRawTransactionStatus, RpcError
>;
#[derive(CandidType, Deserialize, Debug)]
pub enum MultiSendRawTransactionResult {
  Consistent(SendRawTransactionResult),
  Inconsistent(Vec<(RpcService,SendRawTransactionResult,)>),
}
#[derive(CandidType, Deserialize, Debug)]
pub struct Metrics {
  pub responses: Vec<((String,String,String,),u64,)>,
  #[serde(rename="errMaxResponseSizeExceeded")]
  pub err_max_response_size_exceeded: Vec<((String,String,),u64,)>,
  #[serde(rename="inconsistentResponses")]
  pub inconsistent_responses: Vec<((String,String,),u64,)>,
  #[serde(rename="cyclesCharged")]
  pub cycles_charged: Vec<((String,String,),candid::Nat,)>,
  pub requests: Vec<((String,String,),u64,)>,
  #[serde(rename="errHttpOutcall")]
  pub err_http_outcall: Vec<((String,String,RejectionCode,),u64,)>,
}
#[derive(CandidType, Deserialize, Debug)]
pub enum RpcAuth {
  BearerToken{ url: String },
  UrlParameter{ #[serde(rename="urlPattern")] url_pattern: String },
}
#[derive(CandidType, Deserialize, Debug)]
pub enum RpcAccess {
  Authenticated{
    #[serde(rename="publicUrl")]
    public_url: Option<String>,
    auth: RpcAuth,
  },
  Unauthenticated{ #[serde(rename="publicUrl")] public_url: String },
}
#[derive(CandidType, Deserialize, Debug)]
pub struct Provider {
  pub access: RpcAccess,
  pub alias: Option<RpcService>,
  #[serde(rename="chainId")]
  pub chain_id: ChainId,
  #[serde(rename="providerId")]
  pub provider_id: ProviderId,
}
pub type RequestResult = std::result::Result<String, RpcError>;
pub type RequestCostResult = std::result::Result<candid::Nat, RpcError>;

pub struct Service(pub Principal);
impl Service {
  pub async fn eth_call(&self, arg0: &RpcServices, arg1: &Option<RpcConfig>, arg2: &CallArgs) -> Result<(MultiCallResult,)> {
    ic_cdk::call(self.0, "eth_call", (arg0,arg1,arg2,)).await
  }
  pub async fn eth_fee_history(&self, arg0: &RpcServices, arg1: &Option<RpcConfig>, arg2: &FeeHistoryArgs) -> Result<(MultiFeeHistoryResult,)> {
    ic_cdk::call(self.0, "eth_feeHistory", (arg0,arg1,arg2,)).await
  }
  pub async fn eth_get_block_by_number(&self, arg0: &RpcServices, arg1: &Option<RpcConfig>, arg2: &BlockTag) -> Result<(MultiGetBlockByNumberResult,)> {
    ic_cdk::call(self.0, "eth_getBlockByNumber", (arg0,arg1,arg2,)).await
  }
  pub async fn eth_get_logs(&self, arg0: &RpcServices, arg1: &Option<GetLogsRpcConfig>, arg2: &GetLogsArgs) -> Result<(MultiGetLogsResult,)> {
    ic_cdk::call(self.0, "eth_getLogs", (arg0,arg1,arg2,)).await
  }
  pub async fn eth_get_transaction_count(&self, arg0: &RpcServices, arg1: &Option<RpcConfig>, arg2: &GetTransactionCountArgs) -> Result<(MultiGetTransactionCountResult,)> {
    ic_cdk::call(self.0, "eth_getTransactionCount", (arg0,arg1,arg2,)).await
  }
  pub async fn eth_get_transaction_receipt(&self, arg0: &RpcServices, arg1: &Option<RpcConfig>, arg2: &String) -> Result<(MultiGetTransactionReceiptResult,)> {
    ic_cdk::call(self.0, "eth_getTransactionReceipt", (arg0,arg1,arg2,)).await
  }
  pub async fn eth_send_raw_transaction(&self, arg0: &RpcServices, arg1: &Option<RpcConfig>, arg2: &String) -> Result<(MultiSendRawTransactionResult,)> {
    ic_cdk::call(self.0, "eth_sendRawTransaction", (arg0,arg1,arg2,)).await
  }
  /// DEBUG endpoint to retrieve metrics accumulated by the EVM RPC canister.
  /// NOTE: this method exists for debugging purposes, backward compatibility is not guaranteed.
  pub async fn get_metrics(&self) -> Result<(Metrics,)> {
    ic_cdk::call(self.0, "getMetrics", ()).await
  }
  pub async fn get_nodes_in_subnet(&self) -> Result<(u32,)> {
    ic_cdk::call(self.0, "getNodesInSubnet", ()).await
  }
  pub async fn get_providers(&self) -> Result<(Vec<Provider>,)> {
    ic_cdk::call(self.0, "getProviders", ()).await
  }
  pub async fn get_service_provider_map(&self) -> Result<(Vec<(RpcService,ProviderId,)>,)> {
    ic_cdk::call(self.0, "getServiceProviderMap", ()).await
  }
  pub async fn request(&self, arg0: &RpcService, arg1: &String, arg2: &u64) -> Result<(RequestResult,)> {
    ic_cdk::call(self.0, "request", (arg0,arg1,arg2,)).await
  }
  pub async fn request_cost(&self, arg0: &RpcService, arg1: &String, arg2: &u64) -> Result<(RequestCostResult,)> {
    ic_cdk::call(self.0, "requestCost", (arg0,arg1,arg2,)).await
  }
  pub async fn update_api_keys(&self, arg0: &Vec<(ProviderId,Option<String>,)>) -> Result<()> {
    ic_cdk::call(self.0, "updateApiKeys", (arg0,)).await
  }
}

