use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw20::Cw20Coin;

use cosmwasm_std::{Binary, Coin, Timestamp, Uint128};

use crate::state::{AtomicSwapOrder, Status};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ExecuteMsg {
    AddValue {
        amount_traded: Uint128
    },
    SetContract {
        address: String
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Returns volume between specific interval
    VolumeInterval {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Returns total volume till latest timestamp
    TotalVolume {},
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
    BidDetailsbyOrder {
        start_after: Option<String>,
        limit: Option<u32>,
        order_id: String,
    },
    BidDetailsbyBidder {
        order_id: String,
        bidder: String,
    }
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
