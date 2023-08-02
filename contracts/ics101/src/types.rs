use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Binary, Coin, Decimal, Uint128};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateChange {
    #[serde(rename = "In")]
    pub in_tokens: Option<Vec<Coin>>,
    #[serde(rename = "Out")]
    pub out_tokens: Option<Vec<Coin>>,
    #[serde(rename = "PoolTokens")]
    pub pool_tokens: Option<Vec<Coin>>,
    #[serde(rename = "PoolId")]
    pub pool_id: Option<String>,
    #[serde(rename = "MultiDepositOrderId")]
    pub multi_deposit_order_id: Option<String>,
    #[serde(rename = "SourceChainId")]
    pub source_chain_id: Option<String>,
    #[serde(rename = "Shares")]
    pub shares: Option<Uint128>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InterchainSwapPacketData {
    #[serde(rename = "Type")]
    pub r#type: InterchainMessageType,
    #[serde(rename = "Data")]
    pub data: Binary,
    #[serde(rename = "StateChange")]
    pub state_change: Option<Binary>,
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

pub const MULTI_DEPOSIT_PENDING_LIMIT: u64 = 10;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    Pending = 0,
    Complete = 1,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MultiAssetDepositOrder {
    pub order_id: String,
    pub pool_id: String,
    //chain_id: String,
    pub source_maker: String,
    pub destination_taker: String,
    pub deposits: Vec<Coin>,
    //pub pool_tokens: Vec<Coin>,
    pub status: OrderStatus,
    pub created_at: u64
}

/// ## Description - This struct describes a asset (native or CW20) and its normalized weight
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WeightedAsset {
    /// Information about an asset stored in a [`Asset`] struct
    pub asset: Coin,
    /// The weight of the asset
    pub weight: Decimal,
}
