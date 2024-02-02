use cosmwasm_std::{CosmosMsg, CustomMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
/// A number of Custom messages that can call into the side bindings
pub enum SideMsg {
    Swap {
        pool_id: String,
        token_in: String,
        token_out: String,
        slippage: String,
    },
}

impl SideMsg {}

impl From<SideMsg> for CosmosMsg<SideMsg> {
    fn from(msg: SideMsg) -> CosmosMsg<SideMsg> {
        CosmosMsg::Custom(msg)
    }
}

impl CustomMsg for SideMsg {}