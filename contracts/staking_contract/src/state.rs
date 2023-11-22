use cw_storage_plus::Item;

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Uint128};

pub const STATE: Item<State> = Item::new("state");

pub const SEJUNO_FROZEN_TOTAL_ONCHAIN: Item<Uint128> = Item::new("sejuno_frozen_total_onchain");
pub const SEJUNO_FROZEN_TOKENS: Item<Uint128> = Item::new("sejuno_frozen_tokens");

pub const BJUNO_FROZEN_TOTAL_ONCHAIN: Item<Uint128> = Item::new("bjuno_frozen_total_onchain");
pub const BJUNO_FROZEN_TOKENS: Item<Uint128> = Item::new("bjuno_frozen_tokens");


#[derive(Serialize, Debug, Deserialize, Clone, PartialEq)]
pub struct State {
    pub sejuno_backing: Uint128,    // amount of JUNO backing seJUNO in circulation
    pub bjuno_backing: Uint128,     // amount of JUNO backing bJUNO in circulation
    pub to_deposit: Uint128, // amount of JUNO to be deposited but not yet deposited to validators
    pub not_redeemed: Uint128, // amount of JUNO matured but not redeemed by user
    pub sejuno_under_withdraw: Uint128, // amount of seJUNO under 21 days withdraw
    pub bjuno_under_withdraw: Uint128, // amount of JUNO under 21 days withdraw
    pub juno_under_withdraw: Uint128,
    pub sejuno_to_burn: Uint128,
    pub bjuno_to_burn: Uint128,
}

// for reward contrct's global index execution
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RewardExecuteMsg {
    UpdateGlobalIndex {}
}
