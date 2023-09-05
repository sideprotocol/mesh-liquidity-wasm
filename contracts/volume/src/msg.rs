use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Uint128};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ExecuteMsg {
    LogObservation {
        amount_traded: Uint128
    },
    SetContract {
        address: String
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Returns volume between specific interval
    VolumeInterval {
        start: u64,
        end: u64,
    },
    /// Returns total volume till latest timestamp
    TotalVolume {},
    /// Returns total volume till given timestamp
    TotalVolumeAt {
        timestamp: u64
    },
    /// Returns contract address for which volume is tracked
    Contract {},
}

// #[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
// pub struct DetailsResponse {
//     pub id: String,
//     pub maker: MakeSwapMsg,
//     pub status: Status,
//     pub path: String,
//     pub taker: Option<TakeSwapMsg>,
//     pub cancel_timestamp: Option<Timestamp>,
//     pub complete_timestamp: Option<Timestamp>,
// }
// #[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
// pub struct ListResponse {
//     /// List all open swap ids
//     pub swaps: Vec<AtomicSwapOrder>,
// }
