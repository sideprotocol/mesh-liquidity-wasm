use cosmwasm_std::Coin;
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
pub struct ObservationOutput {
    // timestamp
    pub block_timestamp: u64,
    // Number of observations till block_timestamp
    pub num_of_observations: u64,
    // volume cumulative token1
    pub volume1: Coin,
    // volume cumulative token2
    pub volume2: Coin,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
    // admin
    pub admin: String,
    // can only be called by contract
    pub contract_address: String,
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

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Order {
    pub token1: String,
    pub token2: String,
}

pub const OBSERVATIONS: Map<u64, Observation> = Map::new("observations");

pub const CONFIG: Item<Config> = Item::new("config");

pub const ORDER: Item<Order> = Item::new("order");

