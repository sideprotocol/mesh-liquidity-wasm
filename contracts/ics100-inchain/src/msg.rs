use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw20::Cw20Coin;

use cosmwasm_std::{Binary, Coin, Timestamp, Uint128};

use crate::state::{AtomicSwapOrder, Bid, BidStatus, Status};

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
pub struct MigrateMsg {}

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
    #[serde(rename = "TYPE_MSG_MAKE_BID")]
    MakeBid = 4,
    #[serde(rename = "TYPE_MSG_TAKE_BID")]
    TakeBid = 5,
    #[serde(rename = "TYPE_MSG_CANCEL_BID")]
    CancelBid = 6,
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
    /// the tokens to be sold
    pub sell_token: Coin,
    pub buy_token: Coin,
    /// the sender address
    pub maker_address: String,
    /// if desired_taker is specified,
    /// only the desired_taker is allowed to take this order
    pub desired_taker: String,
    /// Allow makers to receive bids for the order
    pub take_bids: bool,
    /// Minimum price required to create bid for this order.
    pub min_bid_price: Option<Uint128>,
    pub expiration_timestamp: u64,
}

impl fmt::Display for MakeSwapMsg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{\"sell_token\":{{\"denom\":\"{}\",\"amount\":\"{}\"}},\"buy_token\":{{\"denom\":\"{}\",\"amount\":\"{}\"}},\"maker_address\":\"{}\",\"desired_taker\":\"{}\",\"expiration_timestamp\":\"{}\"}}",
            self.sell_token.denom,
            self.sell_token.amount,
            self.buy_token.denom,
            self.buy_token.amount,
            self.maker_address,
            self.desired_taker,
            self.expiration_timestamp
        )
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct MakeSwapMsgOutput {
    pub sell_token: Coin,
    pub buy_token: Coin,
    pub maker_address: String,
    pub desired_taker: String,
    pub take_bids: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct TakeSwapMsg {
    pub order_id: String,

    /// the tokens to be sell
    pub sell_token: Coin,
    /// the sender address
    pub taker_address: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct TakeSwapMsgOutput {
    pub order_id: String,
    pub sell_token: Coin,
    pub taker_address: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct CancelSwapMsg {
    pub order_id: String,
    pub maker_address: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct MakeBidMsg {
    pub order_id: String,
    pub sell_token: Coin,
    pub taker_address: String,
    pub expiration_timestamp: u64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct TakeBidMsg {
    pub order_id: String,
    pub bidder: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct CancelBidMsg {
    pub order_id: String,
    pub bidder: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub enum BalanceHuman {
    Native(Vec<Coin>),
    Cw20(Cw20Coin),
}

/// Offset for bid pagination
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct BidOffset {
    pub amount: Uint128,
    pub bidder: String,
}

/// Time Offset for bid pagination
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct BidOffsetTime {
    pub time: u64,
    pub bidder: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct BidOffsetBidder {
    pub order: String,
    pub bidder: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Show all open swaps. Return type is ListResponse.
    List {
        start_after: Option<u64>,
        limit: Option<u32>,
        order: Option<String>,
    },
    ListByDesiredTaker {
        start_after: Option<u64>,
        limit: Option<u32>,
        desired_taker: String,
    },
    ListByMaker {
        start_after: Option<u64>,
        limit: Option<u32>,
        maker: String,
    },
    ListByTaker {
        start_after: Option<u64>,
        limit: Option<u32>,
        taker: String,
    },
    /// Returns the details of the named swap, error if not created.
    /// Return type: DetailsResponse.
    Details {
        id: String,
    },
    BidByAmount {
        order: String,
        status: BidStatus,
        start_after: Option<BidOffset>,
        limit: Option<u32>,
    },
    BidByAmountReverse {
        order: String,
        status: BidStatus,
        start_before: Option<BidOffset>,
        limit: Option<u32>,
    },
    BidbyOrder {
        order: String,
        status: BidStatus,
        start_after: Option<BidOffsetTime>,
        limit: Option<u32>,
    },
    BidbyOrderReverse {
        order: String,
        status: BidStatus,
        start_before: Option<BidOffsetTime>,
        limit: Option<u32>,
    },
    BidDetails {
        order: String,
        bidder: String,
    },
    BidByBidder {
        bidder: String,
        status: BidStatus,
        start_after: Option<String>, // order
        limit: Option<u32>,
    },
    /// Inactive fields query
    InactiveList {
        start_after: Option<u64>,
        limit: Option<u32>,
        order: Option<String>,
    },
    InactiveListByDesiredTaker {
        start_after: Option<u64>,
        limit: Option<u32>,
        desired_taker: String,
    },
    InactiveListByMaker {
        start_after: Option<u64>,
        limit: Option<u32>,
        maker: String,
    },
    InactiveListByTaker {
        start_after: Option<u64>,
        limit: Option<u32>,
        taker: String,
    },
    /// Reverse order.
    ListReverse {
        start_before: Option<u64>,
        limit: Option<u32>,
    },
    ListByDesiredTakerReverse {
        start_before: Option<u64>,
        limit: Option<u32>,
        desired_taker: String,
    },
    ListByMakerReverse {
        start_before: Option<u64>,
        limit: Option<u32>,
        maker: String,
    },
    ListByTakerReverse {
        start_before: Option<u64>,
        limit: Option<u32>,
        taker: String,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct DetailsResponse {
    pub id: String,
    pub maker: MakeSwapMsg,
    pub status: Status,
    pub taker: Option<TakeSwapMsg>,
    pub cancel_timestamp: Option<Timestamp>,
    pub complete_timestamp: Option<Timestamp>,
}
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ListResponse {
    /// List all open swap ids
    pub swaps: Vec<AtomicSwapOrder>,

    pub last_order_id: u64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct BidsResponse {
    pub bids: Vec<Bid>,
}
