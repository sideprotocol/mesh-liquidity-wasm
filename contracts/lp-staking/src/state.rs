use cosmwasm_std::{Addr, Decimal, Uint128, Uint64};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Observation {
    // timestamp
    pub block_timestamp: u64,
    // Number of observations till block_timestamp
    pub num_of_observations: u64,
    // volume cumulative token1
    pub volume1: u128,
    // volume cumulative token2
    pub volume2: u128,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
    // admin
    pub admin: Addr,
    // cw20 token which can be accepted by this contract
    pub deposit_token: Addr,
    // Reward token
    pub reward_token: Addr,
    // distribution rate for reward token
    pub tokens_per_block: u64,
    // alloc points for a token
    pub total_alloc_point: u64,
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
    /// Total virtual amount
    pub total_virtual_supply: Uint128,
}

pub const CONFIG: Item<Config> = Item::new("config");

pub const POOL_INFO: Map<&Addr, PoolInfo> = Map::new("pool_info");
