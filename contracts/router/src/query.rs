use cosmwasm_std::CustomQuery;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::route::SideRoute;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SideQueryWrapper {
    pub route: SideRoute,
    pub query_data: SideQuery,
}

/// SideQuery is an override of QueryRequest::Custom to access Side-specific modules
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SideQuery {
    // Authz
    Grants {
        granter: String,
        grantee: String,
        msg_type_url: String,
        pagination: Option<u32>,
    },
    GranteeGrants {
        grantee: String,
        pagination: Option<u32>,
    },
    GranterGrants {
        granter: String,
        pagination: Option<u32>,
    },
    // Exchange
    Params {},
    Pool {},
    // Pools {
    // }
}

impl CustomQuery for SideQueryWrapper {}