use cosmwasm_std::{Addr, Decimal, Uint128, Uint64};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
    // admin
    pub admin: Addr,
    // Reward token
    pub reward_token: Addr,
    // distribution rate for reward token
    pub tokens_per_block: Uint128,
    // alloc points for a token
    pub total_alloc_point: Uint128,
    // start block
    pub start_block: u64,
    /// The list of active pools with allocation points
    pub active_pools: Vec<(Addr, Uint128)>,
}

/// This structure describes the main information of pool
#[derive(Serialize, Deserialize, PartialEq)]
pub struct PoolInfo {
    /// Accumulated amount of reward per share unit. Used for reward calculations
    pub last_reward_block: Uint64,
    pub reward_global_index: Decimal,
    pub has_asset_rewards: bool,
    pub total_supply: Uint128,
}

/// This structure stores the outstanding amount of token rewards that a user accured.
#[derive(Serialize, Deserialize, PartialEq, Default)]
pub struct UserInfo {
    /// The amount of LP tokens staked
    pub amount: Uint128,
    /// The amount of veToken rewards a user already received or is not eligible for; used for proper reward calculation
    pub reward_user_index: Decimal,
}

pub const CONFIG: Item<Config> = Item::new("config");

pub const POOL_INFO: Map<&Addr, PoolInfo> = Map::new("pool_info");

pub const USER_INFO: Map<&(Addr, Addr), UserInfo> = Map::new("user_info");
