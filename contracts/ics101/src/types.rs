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
pub struct InterchainSwapPacketData {
    #[serde(rename = "Type")]
    pub r#type: InterchainMessageType,
    #[serde(rename = "Data")]
    pub data: Binary,
    #[serde(rename = "StateChange")]
    pub state_change: Option<StateChange>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum InterchainMessageType {
    #[serde(rename = "TYPE_UNSPECIFIED")]
    Unspecified = 0,
    #[serde(rename = "TYPE_MAKE_POOL")]
    MakePool = 1,
    #[serde(rename = "TYPE_TAKE_POOL")]
    TakePool = 2,
    #[serde(rename = "TYPE_SINGLE_ASSET_DEPOSIT")]
    SingleAssetDeposit = 3,
    #[serde(rename = "TYPE_MAKE_MULTI_DEPOSIT")]
    MakeMultiDeposit = 4,
    #[serde(rename = "TYPE_TAKE_MULTI_DEPOSIT")]
    TakeMultiDeposit = 5,
    #[serde(rename = "TYPE_MULTI_WITHDRAW")]
    MultiWithdraw = 6,
    #[serde(rename = "TYPE_LEFT_SWAP")]
    LeftSwap = 7,
    #[serde(rename = "TYPE_RIGHT_SWAP")]
    RightSwap = 8,
}
