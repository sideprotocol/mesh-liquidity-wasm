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

#[derive(Serialize, Deserialize)]
pub struct Forward {
    pub port: String,
    pub channel: String,
    pub timeout: String,
    pub retries: i32,
    #[serde(skip_serializing_if = "Option::is_none")] // This line is to skip serialization if next is None
    pub next: Option<String>, // Optional because it seems to be a comment in your example
}

#[derive(Serialize, Deserialize)]
pub struct Memo {
    pub forward: Forward,
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InterchainSwapPacketData {
    #[serde(rename = "Type")]
    pub r#type: InterchainMessageType,
    #[serde(rename = "Data")]
    pub data: Binary,
    #[serde(rename = "StateChange")]
    pub state_change: Option<Binary>,
    #[serde(rename = "Memo")]
    pub memo: Option<Binary>
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum InterchainMessageType {
    #[serde(rename = "UNSPECIFIED")]
    Unspecified = 0,
    #[serde(rename = "MAKE_POOL")]
    MakePool = 1,
    #[serde(rename = "TAKE_POOL")]
    TakePool = 2,
    #[serde(rename = "CANCEL_POOL")]
    CancelPool = 3,
    #[serde(rename = "SINGLE_ASSET_DEPOSIT")]
    SingleAssetDeposit = 4,
    #[serde(rename = "MAKE_MULTI_DEPOSIT")]
    MakeMultiDeposit = 5,
    #[serde(rename = "CANCEL_MULTI_DEPOSIT")]
    CancelMultiDeposit = 6,
    #[serde(rename = "TAKE_MULTI_DEPOSIT")]
    TakeMultiDeposit = 7,
    #[serde(rename = "MULTI_WITHDRAW")]
    MultiWithdraw = 8,
    #[serde(rename = "LEFT_SWAP")]
    LeftSwap = 9,
    #[serde(rename = "RIGHT_SWAP")]
    RightSwap = 10,
}

pub const MULTI_DEPOSIT_PENDING_LIMIT: u64 = 10;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    Pending = 0,
    Complete = 1,
    Cancelled = 2,
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
    pub created_at: u64,
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
