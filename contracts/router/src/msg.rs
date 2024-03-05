use cosmwasm_std::{Addr, Coin, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ExecuteMsg {
    MultiSwap {
        requests: Vec<SwapRequest>,
        offer_amount: Uint128,
        receiver: Option<Addr>,
        minimum_receive: Option<Uint128>,
    },
    Callback(CallbackMsg)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum CallbackMsg {
    HopSwap {
        requests: Vec<SwapRequest>,
        offer_asset: String,
        prev_ask_amount: Uint128,
        recipient: Addr,
        minimum_receive: Uint128,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SwapRequest {
    /// Pool Id via which the swap is to be routed
    pub pool_id: String,
    /// The offer asset denom
    pub asset_in: String,
    ///  The ask asset denom
    pub asset_out: String,
    /// Contract address, if interchain request
    pub contract_address: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub enum InterchainExecuteMsg {
    MsgSwapRequest {
        #[serde(rename = "swapType")]
        swap_type: SwapMsgType,
        sender: String,
        #[serde(rename = "poolId")]
        pool_id: String,
        #[serde(rename = "tokenIn")]
        token_in: Coin,
        #[serde(rename = "tokenOut")]
        token_out: Coin,
        slippage: u64,
        recipient: String,
        #[serde(rename = "timeoutHeight")]
        timeout_height: u64,
        #[serde(rename = "timeoutTimestamp")]
        timeout_timestamp: u64,
        route: Option<SwapRoute>,
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct SwapRoute {
    pub requests: Vec<SwapRequest>,
    pub minimum_receive: Option<Uint128>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub enum SwapMsgType {
    LEFT = 0,
    RIGHT = 1,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // GetCount returns the current count as a json-encoded number
    GetCount {},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CountResponse {
    pub count: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Params {
    pub pool_creation_fee: u64,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ParamResponse {
    pub params: Params,
}
