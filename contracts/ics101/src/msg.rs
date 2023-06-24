use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Coin, Response, StdError};

use crate::error::ContractError;
use crate::market::{InterchainLiquidityPool, InterchainMarketMaker, PoolAsset};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ExecuteMsg {
    MakePool(MsgMakePoolRequest),
    TakePool(MsgTakePoolRequest),
    SingleAssetDeposit(MsgSingleAssetDepositRequest),
    MakeMultiAssetDeposit(MsgMakeMultiAssetDepositRequest),
    TakeMultiAssetDeposit(MsgTakeMultiAssetDepositRequest),
    //SingleAssetWithdraw(MsgSingleAssetWithdrawRequest),
    MultiAssetWithdraw(MsgMultiAssetWithdrawRequest),
    Swap(MsgSwapRequest),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MsgMakePoolRequest {
    pub source_port: String,
    pub source_channel: String,
    pub creator: String,
    pub counterparty_creator: String,
    pub liquidity:  Vec<PoolAsset>,
    pub swap_fee: u32,
    pub timeout_height: u64,
    pub timeout_timestamp: u64
}

impl MsgMakePoolRequest {
    pub fn validate_basic(&self) -> Result<Response, ContractError> {
        let denom_size = self.liquidity.len();
        // // validation message
        if denom_size != 2 {
            return Err(ContractError::InvalidDenomPair);
        }

        let mut total_weight: u32 = 0;

        for i in 0..self.liquidity.len() {
            total_weight += self.liquidity[i].weight;
        }

        if total_weight != 100 {
            return Err(ContractError::InvalidWeightPair.into());
        }

        Ok(Response::default())
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MsgMakePoolResponse {
    pool_id: String
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MsgTakePoolRequest {
    pub creator: String,
    pub pool_id: String,
    pub timeout_height: u64,
    pub timeout_timestamp: u64
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MsgSingleAssetDepositRequest {
    pub pool_id: String,
    pub sender: String,
    pub token: Coin,
    pub timeout_height: u64,
    pub timeout_timestamp: u64
}

impl MsgSingleAssetDepositRequest {
    pub fn validate_basic(&self) -> Result<Response, ContractError> {
        if self.token.amount.is_zero() {
            return Err(ContractError::Std(StdError::generic_err(
                "Invalid token amount",
            )));
        }

        Ok(Response::default())
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MsgSingleAssetDepositResponse {
    pub pool_token: Coin
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DepositAsset {
    pub sender: String,
    pub balance: Coin
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MsgMakeMultiAssetDepositRequest {
    pub pool_id: String,
    pub deposits: Vec<DepositAsset>,
    pub timeout_height: u64,
    pub timeout_timestamp: u64
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MsgTakeMultiAssetDepositRequest {
    pub sender: String,
    pub pool_id: String,
    pub order_id: u64,
    pub timeout_height: u64,
    pub timeout_timestamp: u64
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MsgMultiAssetDepositResponse {
    pub pool_token: Vec<Coin>
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawAsset {
    pub receiver: String,
    pub balance: Coin
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MsgMultiAssetWithdrawRequest {
    pub pool_id: String,
    pub receiver: String,
    pub counterparty_receiver: String,
    pub pool_token: Coin,
    pub timeout_height: u64,
    pub timeout_timestamp: u64
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MsgMultiAssetWithdrawResponse {
    pub tokens: Vec<Coin>
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MsgSingleAssetWithdrawRequest {
    pub sender: String,
    pub denom_out: String,
    pub pool_coin: Coin,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SwapMsgType {
    Left = 0,
    Right = 1,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct MsgSwapRequest {
    pub swap_type: SwapMsgType,
    pub sender: String,
    #[serde(rename = "poolId")]
    pub pool_id: String,
    #[serde(rename = "tokenIn")]
    pub token_in: Coin,
    #[serde(rename = "tokenOut")]
    pub token_out: Coin,
    pub slippage: u64,
    pub recipient: String,
    #[serde(rename = "timeoutHeight")]
    pub timeout_height: u64,
    #[serde(rename = "timeoutTimestamp")]
    pub timeout_timestamp: u64
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct PoolApprove {
    pub pool_id: String,
    pub sender: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum QueryMsg {
    /// Show all open swaps. Return type is ListResponse.
    List {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    ListByDesiredTaker {
        start_after: Option<String>,
        limit: Option<u32>,
        desired_taker: String,
    },
    /// Returns the details of the named swap, error if not created.
    /// Return type: DetailsResponse.
    Details { id: String },
}

// QueryParamsRequest is the request type for the Query/Params RPC method.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct QueryParamsRequest {}

// QueryParamsResponse is the response type for the Query/Params RPC method.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct QueryParamsResponse {
    pub params: Params,
}

// QueryEscrowAddressRequest is the request type for the EscrowAddress RPC method.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct QueryEscrowAddressRequest {
    pub port_id: String,
    pub channel_id: String,
}

// QueryEscrowAddressResponse is the response type of the EscrowAddress RPC method.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct QueryEscrowAddressResponse {
    pub escrow_address: String,
}

// QueryGetInterchainLiquidityPoolRequest is the request type for the GetInterchainLiquidityPool RPC method.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct QueryGetInterchainLiquidityPoolRequest {
    pub pool_id: String,
}

// QueryGetInterchainLiquidityPoolResponse is the response type for the GetInterchainLiquidityPool RPC method.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct QueryGetInterchainLiquidityPoolResponse {
    pub interchain_liquidity_pool: InterchainLiquidityPool,
}

// QueryAllInterchainLiquidityPoolRequest is the request type for the Query/AllInterchainLiquidityPool RPC method.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct QueryAllInterchainLiquidityPoolRequest {
    pub pagination: PageRequest,
}

// QueryAllInterchainLiquidityPoolResponse is the response type for the Query/AllInterchainLiquidityPool RPC method.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct QueryAllInterchainLiquidityPoolResponse {
    pub interchain_liquidity_pool: Vec<InterchainLiquidityPool>,
    pub pagination: PageResponse,
}

// QueryGetInterchainMarketMakerRequest is the request type for the GetInterchainMarketMaker RPC method.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct QueryGetInterchainMarketMakerRequest {
    pub pool_id: String,
}

// QueryGetInterchainMarketMakerResponse is the response type for the GetInterchainMarketMaker RPC method.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct QueryGetInterchainMarketMakerResponse {
    pub interchain_market_maker: InterchainMarketMaker,
}

// QueryAllInterchainMarketMakerRequest is the request type for the Query/AllInterchainMarketMaker RPC method.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct QueryAllInterchainMarketMakerRequest {
    pub pagination: PageRequest,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct QueryAllInterchainMarketMakerResponse {
    pub interchain_market_maker: Vec<InterchainMarketMaker>,
    pub pagination: PageResponse,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Params {
    // swap_enabled enables or disables all cross-chain token transfers from this chain.
    #[serde(rename = "swap_enabled")]
    pub swap_enabled: bool,
    // max_fee_rate set a max value of fee, it's base point, 1/10000
    #[serde(rename = "max_fee_rate")]
    pub max_fee_rate: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PageRequest {
    #[serde(rename = "key")]
    pub key: Vec<u8>,

    #[serde(rename = "offset")]
    pub offset: u64,

    #[serde(rename = "limit")]
    pub limit: u64,

    #[serde(rename = "count_total")]
    pub count_total: bool,

    #[serde(rename = "reverse")]
    pub reverse: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PageResponse {
    #[serde(rename = "next_key")]
    pub next_key: Vec<u8>,

    #[serde(rename = "total")]
    pub total: u64,
}
