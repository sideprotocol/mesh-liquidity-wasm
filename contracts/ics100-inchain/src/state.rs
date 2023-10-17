use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::{MakeSwapMsg, TakeSwapMsg};
use cosmwasm_std::{Coin, StdResult, Storage, Timestamp, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};

pub const FEE_INFO: Item<FeeInfo> = Item::new("fee_info");
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct FeeInfo {
    // Basis point is 10000
    // so 100 means 100 / 10000 = 1 / 100 = 1% fees of total value
    pub maker_fee: u64,
    pub taker_fee: u64,
    pub treasury: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    Initial, // initialised on maker chain
    Sync,    // synced to the taker chain
    Cancel,  // cancelled
    Failed,
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
    pub maker: MakeSwapMsg,
    pub status: Status,
    pub taker: Option<TakeSwapMsg>,
    // In seconds
    pub create_timestamp: u64,
    pub cancel_timestamp: Option<Timestamp>,
    pub complete_timestamp: Option<Timestamp>,
    pub min_bid_price: Option<Uint128>,
}

pub const SWAP_ORDERS: Map<u64, AtomicSwapOrder> = Map::new("swap_order");
pub const ORDER_TO_COUNT: Map<&str, u64> = Map::new("order_to_count");

pub const COUNT: Item<u64> = Item::new("count");
pub const SWAP_SEQUENCE: Item<u64> = Item::new("swap_sequence");
pub const INACTIVE_COUNT: Item<u64> = Item::new("inactive_count");
pub const INACTIVE_SWAP_ORDERS: Map<u64, AtomicSwapOrder> = Map::new("inactive_swap_order");

// append order to end of list
pub fn append_atomic_order(
    storage: &mut dyn Storage,
    order_id: &str,
    order: &AtomicSwapOrder,
) -> StdResult<u64> {
    let count = COUNT.load(storage)?;

    SWAP_ORDERS.save(storage, count, order)?;
    ORDER_TO_COUNT.save(storage, order_id, &count)?;
    COUNT.save(storage, &(count + 1))?;

    Ok(count)
}

// set specific order
pub fn set_atomic_order(
    storage: &mut dyn Storage,
    order_id: &str,
    order: &AtomicSwapOrder,
) -> StdResult<u64> {
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

/// Move completed or expired order to inactive list
pub fn move_order_to_bottom(storage: &mut dyn Storage, order_id: &str) -> StdResult<u64> {
    // Step 1: Retrieve the item based on the given ID.
    let id: u64 = ORDER_TO_COUNT.load(storage, order_id)?;
    let swap_order = SWAP_ORDERS.load(storage, id)?;
    // Step 2: Remove the item from its current position.
    SWAP_ORDERS.remove(storage, id);
    ORDER_TO_COUNT.remove(storage, order_id);
    // Step 3: Append the item to the end of inactive list.
    let count = INACTIVE_COUNT.load(storage)?;
    INACTIVE_SWAP_ORDERS.save(storage, count, &swap_order)?;
    INACTIVE_COUNT.save(storage, &(count + 1))?;
    Ok(id)
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub enum BidStatus {
    Initial,
    Failed,
    Cancelled,
    Executed,
    Placed,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Bid {
    pub bid: Coin,
    pub order: String,
    pub status: BidStatus,
    pub bidder: String,
    pub receive_timestamp: u64,
    pub expire_timestamp: u64,
}

/// Primary key for asks: (collection, token_id)
pub type BidKey = (String, String);
/// Convenience bid key constructor
pub fn bid_key(order: &String, bidder: &String) -> BidKey {
    (order.clone(), bidder.clone())
}
/// Defines indices for accessing Bids
pub struct BidIndicies<'a> {
    pub order: MultiIndex<'a, String, Bid, BidKey>,
    pub order_price: MultiIndex<'a, (String, u128), Bid, BidKey>,
    pub timestamp: MultiIndex<'a, (String, u64), Bid, BidKey>,
    pub bidder: MultiIndex<'a, String, Bid, BidKey>,
}

impl<'a> IndexList<Bid> for BidIndicies<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Bid>> + '_> {
        let v: Vec<&dyn Index<Bid>> = vec![
            &self.order,
            &self.order_price,
            &self.timestamp,
            &self.bidder,
        ];
        Box::new(v.into_iter())
    }
}

pub fn bids<'a>() -> IndexedMap<'a, BidKey, Bid, BidIndicies<'a>> {
    let indexes = BidIndicies {
        order: MultiIndex::new(|_pk: &[u8], d: &Bid| d.order.clone(), "bids", "bid__order"),
        order_price: MultiIndex::new(
            |_pk: &[u8], d: &Bid| (d.order.clone(), d.bid.amount.u128()),
            "bids",
            "bids__order_price",
        ),
        timestamp: MultiIndex::new(
            |_pk: &[u8], d: &Bid| (d.order.clone(), d.receive_timestamp),
            "bids",
            "bids__count",
        ),
        bidder: MultiIndex::new(
            |_pk: &[u8], d: &Bid| d.bidder.clone(),
            "bids",
            "bid__bidder",
        ),
    };
    IndexedMap::new("bids", indexes)
}
