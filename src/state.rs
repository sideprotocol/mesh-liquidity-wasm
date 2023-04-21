use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::{MakeSwapMsg, TakeSwapMsg};
use cosmwasm_std::{Addr, Binary, BlockInfo, IbcEndpoint, Order, StdResult, Storage, Timestamp};
use cw_storage_plus::{Bound, Map};

use cw20::{Balance, Expiration};

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
    Initial,
    Sync,
    Cancel,
    Complete,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct SwapOrder {
    pub id: String,
    pub maker: MakeSwapMsg,
    pub status: Status,
    pub path: String,
    pub taker: Option<TakeSwapMsg>,
    pub cancel_timestamp: Option<Timestamp>,
    pub complete_timestamp: Option<Timestamp>,
}

pub const SWAP_ORDERS: Map<&str, SwapOrder> = Map::new("swap_order");

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AtomicSwap {
    /// This is the sha-256 hash of the preimage
    pub hash: Binary,
    pub recipient: Addr,
    pub source: Addr,
    pub expires: Expiration,
    /// Balance in native tokens, or cw20 token
    pub balance: Balance,
}

impl AtomicSwap {
    pub fn is_expired(&self, block: &BlockInfo) -> bool {
        self.expires.is_expired(block)
    }
}

pub const SWAPS: Map<&str, AtomicSwap> = Map::new("atomic_swap");

/// This returns the list of ids for all active swaps
pub fn all_swap_ids(
    storage: &dyn Storage,
    start: Option<Bound>,
    limit: usize,
) -> StdResult<Vec<String>> {
    SWAPS
        .keys(storage, start, None, Order::Ascending)
        .take(limit)
        .collect()
}

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

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::MockStorage;
    use cosmwasm_std::Binary;

    #[test]
    fn test_no_swap_ids() {
        let storage = MockStorage::new();
        let ids = all_swap_ids(&storage, None, 10).unwrap();
        assert_eq!(0, ids.len());
    }

    fn dummy_swap() -> AtomicSwap {
        AtomicSwap {
            recipient: Addr::unchecked("recip"),
            source: Addr::unchecked("source"),
            expires: Default::default(),
            hash: Binary("hash".into()),
            balance: Default::default(),
        }
    }

    #[test]
    fn test_all_swap_ids() {
        let mut storage = MockStorage::new();
        SWAPS.save(&mut storage, "lazy", &dummy_swap()).unwrap();
        SWAPS.save(&mut storage, "assign", &dummy_swap()).unwrap();
        SWAPS.save(&mut storage, "zen", &dummy_swap()).unwrap();

        let ids = all_swap_ids(&storage, None, 10).unwrap();
        assert_eq!(3, ids.len());
        assert_eq!(
            vec!["assign".to_string(), "lazy".to_string(), "zen".to_string()],
            ids
        )
    }
}
