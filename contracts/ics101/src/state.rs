use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::IbcEndpoint;
use cw_storage_plus::{Map, Item};

use crate::{market::InterchainLiquidityPool, types::MultiAssetDepositOrder};

pub const CHANNEL_INFO: Map<&str, ChannelInfo> = Map::new("channel_info");

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ChannelInfo {
    /// id of this channel
    pub id: String,
    /// the remote channel/port we connect to
    pub counterparty_endpoint: IbcEndpoint,
    /// the connection this exists on (you can use to query client/consensus info)
    pub connection_id: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
    // Counter to keep track of multiassetdeposit orders
    pub counter: u64,
    // Token code id  (Cw20)
    pub token_code_id: u64
}

// Each pool has it's pool token (cw20)
// Map pool-id -> pool token address
pub const POOL_TOKENS_LIST: Map<&str, String> = Map::new("pool_tokens_list");

pub const CONFIG: Item<Config> = Item::new("config");

pub const POOLS: Map<&str, InterchainLiquidityPool> = Map::new("pools");

// Map from pool id to vec<oders>
pub const MULTI_ASSET_DEPOSIT_ORDERS: Map<String, Vec<MultiAssetDepositOrder>> = Map::new("multi_asset_deposit_orders");

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    Initial,  // initialed on maker chain
    Sync,     // synced to the taker chain
    Cancel,   // canceled
    Complete, // completed
}
