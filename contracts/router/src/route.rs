use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// SideRoute is enum type to represent side query route path
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SideRoute {
    Authz,
    Gmm,
}