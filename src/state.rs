use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::{MakeSwapMsg, TakeSwapMsg};
use cosmwasm_std::{IbcEndpoint, Order, StdResult, Storage, Timestamp};
use cw_storage_plus::{Bound, Map};

pub const CHANNEL_INFO: Map<&str, ChannelInfo> = Map::new("channel_info");

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ChannelInfo {
    /// id of this channel
    pub id: String,
    /// the remote channel/port we connect to
    pub counterparty_endpoint: IbcEndpoint,
    /// the connection this exists on (you can use to query client/consensus info)
    pub connection_id: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    Initial,  // initialed on maker chain
    Sync,     // synced to the taker chain
    Cancel,   // canceled
    Complete, // completed
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct AtomicSwapOrder {
    pub id: String,
    pub maker: MakeSwapMsg,
    pub status: Status,
    // an IBC path, define channel and port on both Maker Chain and Taker Chain
    pub path: String,
    pub taker: Option<TakeSwapMsg>,
    pub cancel_timestamp: Option<Timestamp>,
    pub complete_timestamp: Option<Timestamp>,
}

pub const SWAP_ORDERS: Map<&str, AtomicSwapOrder> = Map::new("swap_order");

pub fn all_swap_order_ids(
    storage: &dyn Storage,
    start: Option<Bound>,
    limit: usize,
) -> StdResult<Vec<String>> {
    SWAP_ORDERS
        .keys(storage, start, None, Order::Ascending)
        .take(limit)
        .collect()
}

pub fn all_swap_orders(
    storage: &dyn Storage,
    start: Option<Bound>,
    limit: usize,
) -> StdResult<Vec<String>> {
    SWAP_ORDERS
        .keys(storage, start, None, Order::Ascending)
        .take(limit)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::MockStorage;

    #[test]
    fn test_no_swap_ids() {
        let storage = MockStorage::new();
        let ids = all_swap_order_ids(&storage, None, 10).unwrap();
        assert_eq!(0, ids.len());
    }
}
