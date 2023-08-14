use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Binary, Coin, Decimal};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateChange {
    #[serde(rename = "in")]
    pub in_tokens: Vec<Option<Coin>>,
    #[serde(rename = "out")]
    pub out_tokens: Vec<Option<Coin>>,
    #[serde(rename = "poolTokens")]
    pub pool_tokens: Vec<Option<Coin>>,
    #[serde(rename = "poolId")]
    pub pool_id: Option<String>,
    #[serde(rename = "multiDepositOrderId")]
    pub multi_deposit_order_id: Option<String>,
    #[serde(rename = "sourceChainId")]
    pub source_chain_id: Option<String>,
    // #[serde(rename = "Shares")]
    // pub shares: Option<Uint128>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct IBCSwapPacketData {
    #[serde(rename = "type")]
    pub r#type: SwapMessageType,
    #[serde(rename = "data")]
    pub data: Binary,
    #[serde(rename = "stateChange")]
    pub state_change: Option<Binary>,
    #[serde(rename = "memo")]
    pub memo: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum SwapMessageType {
    #[serde(rename = "TYPE_UNSPECIFIED")]
    Unspecified = 0,
    #[serde(rename = "TYPE_MAKE_POOL")]
    MakePool = 1,
    #[serde(rename = "TYPE_TAKE_POOL")]
    TakePool = 2,
    #[serde(rename = "TYPE_CANCEL_POOL")]
    CancelPool = 3,
    #[serde(rename = "TYPE_SINGLE_ASSET_DEPOSIT")]
    SingleAssetDeposit = 4,
    #[serde(rename = "TYPE_MAKE_MULTI_DEPOSIT")]
    MakeMultiDeposit = 5,
    #[serde(rename = "TYPE_CANCEL_MULTI_DEPOSIT")]
    CancelMultiDeposit = 6,
    #[serde(rename = "TYPE_TAKE_MULTI_DEPOSIT")]
    TakeMultiDeposit = 7,
    #[serde(rename = "TYPE_MULTI_WITHDRAW")]
    MultiWithdraw = 8,
    #[serde(rename = "TYPE_LEFT_SWAP")]
    LeftSwap = 9,
    #[serde(rename = "TYPE_RIGHT_SWAP")]
    RightSwap = 10,
}

pub const MULTI_DEPOSIT_PENDING_LIMIT: u64 = 10;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    Pending = 0,
    Complete = 1,
    Cancelled = 2
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MultiAssetDepositOrder {
    pub id: String,
    pub pool_id: String,
    pub chain_id: String,
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
