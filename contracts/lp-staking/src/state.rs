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
    pub admin: String,
    // cw20 token which can be accepted by this contract
    pub deposit_token: String,
    // distribution rate for reward token
    pub tokens_per_block: u64,
    // alloc points for a token
    pub total_alloc_point: u64,
    // start block
    pub start_block: u64,
    // Reward token
    pub reward_token: String,
}
pub const OBSERVATIONS: Map<u64, Observation> = Map::new("observations");

pub const CONFIG: Item<Config> = Item::new("config");
