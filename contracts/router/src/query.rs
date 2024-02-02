use cosmwasm_std::CustomQuery;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// SideQuery is an override of QueryRequest::Custom to access Side-specific modules
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SideQuery {
    // Exchange
    Params {},
    Pool {},
    // TODO: Add more queries
    // Pools {
    // }
}

impl CustomQuery for SideQuery {}