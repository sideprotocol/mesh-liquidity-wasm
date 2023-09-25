use cw20::{MinterResponse, Cw20Coin, Logo};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Coin, Response, StdError, StdResult, Uint128};

use crate::error::ContractError;
use crate::market::{InterchainLiquidityPool, InterchainMarketMaker, PoolAsset, PoolStatus};
use crate::types::MultiAssetDepositOrder;
use crate::utils::{is_valid_symbol, is_valid_name};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InstantiateMsg {
    pub token_code_id: u64
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ExecuteMsg {
    MakePool(MsgMakePoolRequest),
    TakePool(MsgTakePoolRequest),
    CancelPool(MsgCancelPoolRequest),
    SingleAssetDeposit(MsgSingleAssetDepositRequest),
    MakeMultiAssetDeposit(MsgMakeMultiAssetDepositRequest),
    CancelMultiAssetDeposit(MsgCancelMultiAssetDepositRequest),
    TakeMultiAssetDeposit(MsgTakeMultiAssetDepositRequest),
    MultiAssetWithdraw(MsgMultiAssetWithdrawRequest),
    Swap(MsgSwapRequest),
    RemovePool(MsgRemovePool),
    SetLogAddress {
        pool_id: String,
        address: String,
    }
   // Receive(Cw20ReceiveMsg)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum Cw20HookMsg {
    WithdrawLiquidity {
        pool_id: String,
        receiver: String,
        counterparty_receiver: String,
        timeout_height: u64,
        timeout_timestamp: u64
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MsgRemovePool {
   pub pool_id: String
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MsgMakePoolRequest {
    pub source_port: String,
    pub source_channel: String,
    pub source_chain_id: String,
    pub destination_chain_id: String,
    pub counterparty_channel: String,
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
            return Err(ContractError::InvalidWeightPair);
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
    pub counter_creator: String,
    pub creator: String,
    pub pool_id: String,
    pub timeout_height: u64,
    pub timeout_timestamp: u64
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MsgCancelPoolRequest {
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
    pub chain_id: String,
    pub timeout_height: u64,
    pub timeout_timestamp: u64
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MsgTakeMultiAssetDepositRequest {
    pub sender: String,
    pub pool_id: String,
    pub order_id: String,
    pub timeout_height: u64,
    pub timeout_timestamp: u64
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MsgCancelMultiAssetDepositRequest {
    pub sender: String,
    pub pool_id: String,
    pub order_id: String,
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
pub enum SwapMsgType {
    LEFT = 0,
    RIGHT = 1,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct MsgSwapRequest {
    #[serde(rename = "swapType")]
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

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InstantiateMarketingInfo {
    /// The project name
    pub project: Option<String>,
    /// The project description
    pub description: Option<String>,
    /// The address of an admin who is able to update marketing info
    pub marketing: Option<String>,
    /// The token logo
    pub logo: Option<Logo>,
}


/// This structure describes the parameters used for creating a token contract.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct TokenInstantiateMsg {
    /// Token name
    pub name: String,
    /// Token symbol
    pub symbol: String,
    pub initial_balances: Vec<Cw20Coin>,
    /// The amount of decimals the token has
    pub decimals: u8,
    /// Minting controls specified in a [`MinterResponse`] structure
    pub mint: Option<MinterResponse>,
    pub marketing: Option<InstantiateMarketingInfo>,
}

impl TokenInstantiateMsg {
    pub fn get_cap(&self) -> Option<Uint128> {
        self.mint.as_ref().and_then(|v| v.cap)
    }

    pub fn validate(&self) -> StdResult<()> {
        // Check name, symbol, decimals
        if !is_valid_name(&self.name) {
            return Err(StdError::generic_err(
                "Name is not in the expected format (3-50 UTF-8 bytes)",
            ));
        }
        if !is_valid_symbol(&self.symbol, None) {
            return Err(StdError::generic_err(
                "Ticker symbol is not in expected format [a-zA-Z\\-]{3,12}",
            ));
        }
        if self.decimals > 18 {
            return Err(StdError::generic_err("Decimals must not exceed 18"));
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub enum LogExecuteMsg {
    LogObservation {
        token1: Coin,
        token2: Coin,
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum QueryMsg {
    /// Show all open orders. Return type is ListResponse.
    OrderList {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    Order {
        pool_id: String,
        order_id: String,
    },
    /// Query config
    Config {},
    /// Query all pool token list
    PoolTokenList {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    PoolAddressByToken {
        pool_id: String
    },
    InterchainPool {
        pool_id: String
    },
    InterchainPoolList {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    LeftSwap {
        pool_id: String,
        token_in: Coin,
        token_out: Coin,
    },
    RightSwap {
        pool_id: String,
        token_in: Coin,
        token_out: Coin,
    },
    QueryActiveOrders {
        source_maker: String,
        destination_taker: String,
        pool_id: String,
    },
    Rate {
        amount: Uint128,
        pool_id: String
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InterchainPoolResponse {
    pub id: String,
    pub source_creator: String,
    pub source_chain_id: String,
    pub destination_chain_id: String,
    pub destination_creator: String,
    pub assets: Vec<PoolAsset>,
    pub swap_fee: u32,
    pub supply: Coin,
    pub status: PoolStatus,
    pub counter_party_port: String,
    pub counter_party_channel: String,
}


#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InterchainListResponse {
    pub pools: Vec<InterchainLiquidityPool>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct OrderListResponse {
    pub orders: Vec<MultiAssetDepositOrder>,
}


#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct PoolListResponse {
    pub pools: Vec<String>,
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

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct QueryConfigResponse {
    /// For order save in state
    pub counter: u64,
    /// For Instantiating cw20 tokens
    pub token_code_id: u64
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
