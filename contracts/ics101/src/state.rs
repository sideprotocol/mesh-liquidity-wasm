use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::IbcEndpoint;
use cw_storage_plus::Map;

use crate::market::InterchainLiquidityPool;

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

pub const POOLS: Map<&str, InterchainLiquidityPool> = Map::new("pools");

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    Initial,  // initialed on maker chain
    Sync,     // synced to the taker chain
    Cancel,   // canceled
    Complete, // completed
}
