use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::{MakeSwapMsg, TakeSwapMsg};
use cosmwasm_std::{IbcEndpoint, Order, StdResult, Storage, Timestamp};
use cw_storage_plus::{Bound, Map, Item};

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
    Initial,  // initialised on maker chain
    Sync,     // synced to the taker chain
    Cancel,   // cancelled
    Complete, // completed
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Side {
    Native,
    Remote,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AtomicSwapOrder {
    pub id: String,
    pub side: Side,
    pub maker: MakeSwapMsg,
    pub status: Status,
    // an IBC path, define channel and port on both Maker Chain and Taker Chain
    pub path: String,
    pub taker: Option<TakeSwapMsg>,
    pub cancel_timestamp: Option<Timestamp>,
    pub complete_timestamp: Option<Timestamp>,
}

pub const SWAP_ORDERS: Map<u64, AtomicSwapOrder> = Map::new("swap_order");
pub const ORDER_TO_COUNT: Map<&str, u64> = Map::new("order_to_count");
pub const COUNT: Item<u64> = Item::new("count");

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

// set specific order
pub fn set_atomic_order(storage: &mut dyn Storage, order_id: &str, order: &AtomicSwapOrder) -> StdResult<u64> {
    let id = ORDER_TO_COUNT.load(storage, order_id)?;
    SWAP_ORDERS.save(storage, id, order)?;
    Ok(id)
}

// set specific order
pub fn get_atomic_order(storage: &dyn Storage, order_id: &str) -> StdResult<AtomicSwapOrder> {
    let id = ORDER_TO_COUNT.load(storage, order_id)?;
    let swap_order = SWAP_ORDERS.load(storage, id)?;
    Ok(swap_order)
}

// set specific order
pub fn remove_atomic_order(storage: &mut dyn Storage, order_id: &str) -> StdResult<u64> {
    let id = ORDER_TO_COUNT.load(storage, order_id)?;
    SWAP_ORDERS.remove(storage, id);
    Ok(id)
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
