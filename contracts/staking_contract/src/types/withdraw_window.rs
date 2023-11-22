use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use serde::{Deserialize, Serialize};

pub const USER_CLAIMABLE: Item<UserClaimable> = Item::new("user_claimable");

pub const QUEUE_WINDOW_AMOUNT: Map<&Addr, Uint128> = Map::new("queue_window");
pub const ONGOING_WITHDRAWS_AMOUNT: Map<(&str, &Addr), Uint128> = Map::new("ongoing_withdraws");
pub const USER_CLAIMABLE_AMOUNT: Map<&Addr, Uint128> = Map::new("user_claimable_map");

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct QueueWindow {
    pub id: u64,
    pub total_lsside: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct OngoingWithdrawWindow {
    pub id: u64,
    pub time_to_mature_window: u64,
    pub total_juno: Uint128,
    pub total_lsside: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct UserClaimable {
    pub total_side: Uint128,
}
