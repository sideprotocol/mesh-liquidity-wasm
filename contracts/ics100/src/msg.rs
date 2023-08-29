use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw20::Cw20Coin;

use cosmwasm_std::{Binary, Coin, Timestamp};

use crate::state::{AtomicSwapOrder, Status};

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
    MakeBid(MakeBidMsg),
    TakeBid(TakeBidMsg),
    CancelBid(CancelBidMsg),
}

pub fn is_valid_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.len() < 3 || bytes.len() > 20 {
        return false;
    }
    true
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {
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
    pub order_id: Option<String>,
    pub path: Option<String>,
    pub memo: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct MakeSwapMsg {
    /// the port on which the packet will be sent
    pub source_port: String,
    /// the channel by which the packet will be sent
    pub source_channel: String,
    /// the tokens to be sold
    pub sell_token: Coin,
    pub buy_token: Coin,
    /// the sender address
    pub maker_address: String,
    /// the sender's address on the destination chain
    pub maker_receiving_address: String,
    /// if desired_taker is specified,
	/// only the desired_taker is allowed to take this order
	/// this is address on destination chain
    pub desired_taker: String,
    /// Allow makers to receive bids for the order
    pub take_bids: bool,

    pub timeout_height: Height,
    pub timeout_timestamp: u64,

    pub expiration_timestamp: u64,
}

impl fmt::Display for MakeSwapMsg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{\"source_port\":\"{}\",\"source_channel\":\"{}\",\"sell_token\":{{\"denom\":\"{}\",\"amount\":\"{}\"}},\"buy_token\":{{\"denom\":\"{}\",\"amount\":\"{}\"}},\"maker_address\":\"{}\",\"maker_receiving_address\":\"{}\",\"desired_taker\":\"{}\",\"timeout_height\":{{\"revision_number\":\"{}\",\"revision_height\":\"{}\"}},\"timeout_timestamp\":\"{}\",\"expiration_timestamp\":\"{}\"}}",
            self.source_port,
            self.source_channel,
            self.sell_token.denom,
            self.sell_token.amount,
            self.buy_token.denom,
            self.buy_token.amount,
            self.maker_address,
            self.maker_receiving_address,
            self.desired_taker,
            self.timeout_height.revision_number,
            self.timeout_height.revision_height,
            self.timeout_timestamp,
            self.expiration_timestamp
        )
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct MakeSwapMsgOutput {
    pub source_port: String,
    pub source_channel: String,
    pub sell_token: Coin,
    pub buy_token: Coin,
    pub maker_address: String,
    pub maker_receiving_address: String,
    pub desired_taker: String,
    pub timeout_height: HeightOutput,
    pub timeout_timestamp: String,
    pub expiration_timestamp: String,
    pub take_bids: bool
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct HeightOutput {
    pub revision_number: String,
    pub revision_height: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct TakeSwapMsg {
    pub order_id: String,

    /// the tokens to be sell
    pub sell_token: Coin,
    /// the sender address
    pub taker_address: String,
    /// the sender's address on the destination chain
    pub taker_receiving_address: String,

    /// Timeout height relative to the current block height.
	/// The timeout is disabled when set to 0.
    pub timeout_height: Height,

    /// Timeout timestamp in absolute nanoseconds since unix epoch.
	/// The timeout is disabled when set to 0.
    pub timeout_timestamp: u64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct TakeSwapMsgOutput {
    pub order_id: String,
    pub sell_token: Coin,
    pub taker_address: String,
    pub taker_receiving_address: String,
    pub timeout_height: HeightOutput,
    pub timeout_timestamp: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct CancelSwapMsg {
    pub order_id: String,

    pub maker_address: String,

    pub timeout_height: HeightOutput,

    pub timeout_timestamp: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct MakeBidMsg {
    pub order_id: String,
    pub sell_token: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct TakeBidMsg {
    pub order_id: String,
    pub bidder: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct CancelBidMsg {
    pub order_id: String,
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
    ListByMaker {
        start_after: Option<String>,
        limit: Option<u32>,
        maker: String,
    },
    ListByTaker {
        start_after: Option<String>,
        limit: Option<u32>,
        taker: String,
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
    pub swaps: Vec<AtomicSwapOrder>,
}
