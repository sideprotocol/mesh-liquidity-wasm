use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw20::Cw20Coin;

use cosmwasm_std::{Binary, Coin, Timestamp};

use crate::state::Status;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Height {
    #[serde(rename = "revision_number")]
    pub revision_number: u64,

    #[serde(rename = "revision_height")]
    pub revision_height: u64,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ExecuteMsg {
    MakeSwap(MakeSwapMsg),
    TakeSwap(TakeSwapMsg),
    CancelSwap(CancelSwapMsg),
}

pub fn is_valid_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.len() < 3 || bytes.len() > 20 {
        return false;
    }
    true
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum SwapMessageType {
    #[serde(rename = "TYPE_UNSPECIFIED")]
    Unspecified = 0,
    #[serde(rename = "TYPE_MSG_MAKE_SWAP")]
    MakeSwap = 1,
    #[serde(rename = "TYPE_MSG_TAKE_SWAP")]
    TakeSwap = 2,
    #[serde(rename = "TYPE_MSG_CANCEL_SWAP")]
    CancelSwap = 3,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AtomicSwapPacketData {
    pub r#type: SwapMessageType,
    pub data: Binary,
    pub memo: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct MakeSwapMsg {
    #[serde(rename = "source_port")]
    pub source_port: String,

    #[serde(rename = "source_channel")]
    pub source_channel: String,

    #[serde(rename = "sell_token")]
    pub sell_token: Coin,

    #[serde(rename = "buy_token")]
    pub buy_token: Coin,

    #[serde(rename = "maker_address")]
    pub maker_address: String,

    #[serde(rename = "maker_receiving_address")]
    pub maker_receiving_address: String,

    #[serde(rename = "desired_taker")]
    pub desired_taker: String,

    #[serde(rename = "create_timestamp")]
    pub create_timestamp: i64,

    #[serde(rename = "timeout_height")]
    pub timeout_height: Height,

    #[serde(rename = "timeout_timestamp")]
    pub timeout_timestamp: u64,

    #[serde(rename = "expiration_timestamp")]
    pub expiration_timestamp: u64,
}

impl fmt::Display for MakeSwapMsg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{\"source_port\":\"{}\",\"source_channel\":\"{}\",\"sell_token\":{{\"denom\":\"{}\",\"amount\":\"{}\"}},\"buy_token\":{{\"denom\":\"{}\",\"amount\":\"{}\"}},\"maker_address\":\"{}\",\"maker_receiving_address\":\"{}\",\"desired_taker\":\"{}\",\"create_timestamp\":\"{}\",\"timeout_height\":{{\"revision_number\":\"{}\",\"revision_height\":\"{}\"}},\"timeout_timestamp\":\"{}\",\"expiration_timestamp\":\"{}\"}}",
            self.source_port,
            self.source_channel,
            self.sell_token.denom,
            self.sell_token.amount,
            self.buy_token.denom,
            self.buy_token.amount,
            self.maker_address,
            self.maker_receiving_address,
            self.desired_taker,
            self.create_timestamp,
            self.timeout_height.revision_number,
            self.timeout_height.revision_height,
            self.timeout_timestamp,
            self.expiration_timestamp
        )
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct MakeSwapMsgOutput {
    #[serde(rename = "source_port")]
    pub source_port: String,

    #[serde(rename = "source_channel")]
    pub source_channel: String,

    #[serde(rename = "sell_token")]
    pub sell_token: Coin,

    #[serde(rename = "buy_token")]
    pub buy_token: Coin,

    #[serde(rename = "maker_address")]
    pub maker_address: String,

    #[serde(rename = "maker_receiving_address")]
    pub maker_receiving_address: String,

    #[serde(rename = "desired_taker")]
    pub desired_taker: String,

    #[serde(rename = "create_timestamp")]
    pub create_timestamp: String,

    #[serde(rename = "timeout_height")]
    pub timeout_height: HeightOutput,

    #[serde(rename = "timeout_timestamp")]
    pub timeout_timestamp: String,

    #[serde(rename = "expiration_timestamp")]
    pub expiration_timestamp: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct HeightOutput {
    pub revision_number: String,
    pub revision_height: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct TakeSwapMsg {
    #[serde(rename = "order_id")]
    pub order_id: String,

    #[serde(rename = "sell_token")]
    pub sell_token: Coin,

    #[serde(rename = "taker_address")]
    pub taker_address: String,

    #[serde(rename = "taker_receiving_address")]
    pub taker_receiving_address: String,

    #[serde(rename = "timeout_height")]
    pub timeout_height: Height,

    #[serde(rename = "timeout_timestamp")]
    pub timeout_timestamp: u64,

    #[serde(rename = "create_timestamp")]
    pub create_timestamp: i64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct TakeSwapMsgOutput {
    #[serde(rename = "order_id")]
    pub order_id: String,

    #[serde(rename = "sell_token")]
    pub sell_token: Coin,

    #[serde(rename = "taker_address")]
    pub taker_address: String,

    #[serde(rename = "taker_receiving_address")]
    pub taker_receiving_address: String,

    #[serde(rename = "timeout_height")]
    pub timeout_height: HeightOutput,

    #[serde(rename = "timeout_timestamp")]
    pub timeout_timestamp: String,

    #[serde(rename = "create_timestamp")]
    pub create_timestamp: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct CancelSwapMsg {
    #[serde(rename = "order_id")]
    pub order_id: String,

    #[serde(rename = "maker_address")]
    pub maker_address: String,

    #[serde(rename = "timeout_height")]
    pub timeout_height: HeightOutput,

    #[serde(rename = "timeout_timestamp")]
    pub timeout_timestamp: String,

    #[serde(rename = "create_timestamp")]
    pub create_timestamp: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub enum BalanceHuman {
    Native(Vec<Coin>),
    Cw20(Cw20Coin),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
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

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct DetailsResponse {
    pub id: String,
    pub maker: MakeSwapMsg,
    pub status: Status,
    pub path: String,
    pub taker: Option<TakeSwapMsg>,
    pub cancel_timestamp: Option<Timestamp>,
    pub complete_timestamp: Option<Timestamp>,
}
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ListResponse {
    /// List all open swap ids
    pub swaps: Vec<DetailsResponse>,
}
