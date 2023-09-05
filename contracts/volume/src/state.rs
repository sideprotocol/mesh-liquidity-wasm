use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_storage_plus::{Item};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Observation {
    // timestamp
    pub block_timestamp: u64,
    // Number of observations till block_timestamp
    pub num_of_observations: u64,
    // volume cumulative
    pub volume: u64
}

pub const OBSERVATIONS: Item<Vec<Observation>> = Item::new("observations");

