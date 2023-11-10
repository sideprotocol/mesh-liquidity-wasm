use cosmwasm_std::{Coin, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct VestingDetails {
    // cliff timestamp: after this timestamp vesting will start
    pub cliff: u64,
    // total time for which token will release
    pub vested_time: u64,
    // Interval after which tokens will be released
    // vested_time % release_interval should be 0
    pub release_interval: u128,
    // token receiver, can claim tokens
    pub receiver: String,
    // total amount of tokens,
    pub token: Coin,
    // total claimed
    pub amount_claimed: Uint128,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
    // admin
    pub admin: String,
    // allowed addresses which can enable vesting for receiver
    pub allowed_addresses: String,
    // current index
    pub current_idx: u64,
    // pivoted or not
    pub pivoted: bool,
    // Maximum length
    pub max_length: u64,
    // Is new
    pub is_new: bool,
    // total observations in map
    pub counter: u64,
}
pub const VESTED_TOKENS_ALL: Map<String, Vec<VestingDetails>> = Map::new("vested_tokens_all");

pub const CONFIG: Item<Config> = Item::new("config");
