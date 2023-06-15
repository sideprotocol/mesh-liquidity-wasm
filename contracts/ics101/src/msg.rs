use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Coin, Response, StdError};

use crate::error::ContractError;
use crate::market::{InterchainLiquidityPool, InterchainMarketMaker};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ExecuteMsg {
    CreatePool(MsgCreatePoolRequest),
    SingleAssetDeposit(MsgSingleAssetDepositRequest),
    MultiAssetDeposit(MsgMultiAssetDepositRequest),
    SingleAssetWithdraw(MsgSingleAssetWithdrawRequest),
    MultiAssetWithdraw(MsgMultiAssetWithdrawRequest),
    Swap(MsgSwapRequest),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MsgCreatePoolRequest {
    pub source_port: String,
    pub source_channel: String,
    pub sender: String,
    pub tokens: Vec<Coin>,
    pub decimals: Vec<u32>,
<<<<<<< HEAD
    pub weights: Vec<u32>,
=======
    pub weight: String,
>>>>>>> 5964b71 (merge)
}

impl MsgCreatePoolRequest {
    pub fn validate_basic(&self) -> Result<Response, ContractError> {
        let denom_size = self.tokens.len();
        // validation message
        if denom_size != 2 {
            return Err(ContractError::InvalidDenomPair);
        }

        if self.decimals.len() != 2 {
            return Err(ContractError::InvalidDecimalPair.into());
        }

<<<<<<< HEAD
        // let weights: Vec<&str> = self.weight.split(':').collect();
        // if weights.len() != 2 {
        //     return Err(ContractError::InvalidWeightPair.into());
        // }

        let mut total_weight: u32 = 0;

        for i in 0..self.weights.len() {
            total_weight += self.weights[i];
=======
        let weights: Vec<&str> = self.weight.split(':').collect();
        if weights.len() != 2 {
            return Err(ContractError::InvalidWeightPair.into());
        }

        let mut total_weight: u32 = 0;

        for i in 0..weights.len() {
            total_weight += weights[i].parse::<u32>().unwrap();
>>>>>>> 5964b71 (merge)
        }

        if total_weight != 100 {
            return Err(ContractError::InvalidWeightPair.into());
        }

        Ok(Response::default())
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MsgSingleAssetDepositRequest {
    pub pool_id: String,
    pub sender: String,
    pub token: Coin,
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
pub struct MsgMultiAssetDepositRequest {
    pub pool_id: String,
    pub local_deposit: LocalDeposit,
    pub remote_deposit: RemoteDeposit,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct LocalDeposit {
    pub sender: String,
    pub token: Coin,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct RemoteDeposit {
    pub sender: String,
    pub token: Coin,
    pub sequence: u64,
    pub signature: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MsgMultiAssetWithdrawRequest {
    pub local_withdraw: MsgSingleAssetWithdrawRequest,
    pub remote_withdraw: MsgSingleAssetWithdrawRequest,
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
    #[serde(rename = "tokenIn")]
    pub token_in: Coin,
    #[serde(rename = "tokenOut")]
    pub token_out: Coin,
    pub slippage: u64,
    pub recipient: String,
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
