use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Binary, Coin};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateChange {
    #[serde(rename = "In")]
    pub in_tokens: Option<Vec<Coin>>,
    #[serde(rename = "Out")]
    pub out_tokens: Option<Vec<Coin>>,
    #[serde(rename = "PoolTokens")]
    pub pool_tokens: Option<Vec<Coin>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct IBCSwapPacketData {
    #[serde(rename = "Type")]
    pub r#type: SwapMessageType,
    #[serde(rename = "Data")]
    pub data: Binary,
    #[serde(rename = "StateChange")]
    pub state_change: Option<StateChange>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum SwapMessageType {
    #[serde(rename = "TYPE_UNSPECIFIED")]
    Unspecified = 0,
    #[serde(rename = "TYPE_CREATE_POOL")]
    CreatePool = 1,
    #[serde(rename = "TYPE_SINGLE_DEPOSIT")]
    SingleDeposit = 2,
    #[serde(rename = "TYPE_MULTI_DEPOSIT")]
    MultiDeposit = 3,
    #[serde(rename = "TYPE_SINGLE_WITHDRAW")]
    SingleWithdraw = 4,
    #[serde(rename = "TYPE_MULTI_WITHDRAW")]
    MultiWithdraw = 5,
    #[serde(rename = "TYPE_LEFT_SWAP")]
    LeftSwap = 6,
    #[serde(rename = "TYPE_RIGHT_SWAP")]
    RightSwap = 7,
}
