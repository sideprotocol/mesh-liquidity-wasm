use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw20::{Balance, Cw20Coin};

use cosmwasm_std::{Binary, Coin, Timestamp};

use crate::state::Status;

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
    #[serde(rename = "TYPE_MSG_MAKE_SWAP")]
    MakeSwap,
    #[serde(rename = "TYPE_MSG_TAKE_SWAP")]
    TakeSwap,
    #[serde(rename = "TYPE_MSG_CANCEL_SWAP")]
    CancelSwap,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AtomicSwapPacketData {
    pub message_type: SwapMessageType,
    pub data: Binary,
    pub memo: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct MakeSwapMsg {
    pub source_port: String,
    pub source_channel: String,
    pub sell_token: Balance,
    pub buy_token: Balance,
    pub maker_address: String,
    pub maker_receiving_address: String,
    pub desired_taker: Option<String>,
    pub creation_timestamp: Timestamp,
    pub expiration_timestamp: Timestamp,
    pub timeout_height: u64,
    pub timeout_timestamp: Timestamp,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct TakeSwapMsg {
    pub order_id: String,
    // the tokens to be sold
    pub sell_token: Balance,
    // the taker's address
    pub taker_address: String,
    // the taker's address on the maker chain
    pub taker_receiving_address: String,
    pub creation_timestamp: Timestamp,
    pub timeout_height: u64,
    pub timeout_timestamp: Timestamp,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct CancelSwapMsg {
    pub order_id: String,
    pub maker_address: String,
    pub creation_timestamp: Timestamp,
    pub timeout_height: u64,
    pub timeout_timestamp: Timestamp,
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
