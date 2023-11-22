use cw_storage_plus::Item;

use serde::{Deserialize, Serialize};

use cosmwasm_std::Uint128;

pub const STATE: Item<State> = Item::new("state");

pub const LSSIDE_FROZEN_TOTAL_ONCHAIN: Item<Uint128> = Item::new("lsside_frozen_total_onchain");
pub const LSSIDE_FROZEN_TOKENS: Item<Uint128> = Item::new("lsside_frozen_tokens");

#[derive(Serialize, Debug, Deserialize, Clone, PartialEq)]
pub struct State {
    pub lsside_backing: Uint128, // amount of SIDE backing lsSIDE in circulation
    pub to_deposit: Uint128, // amount of SIDE to be deposited but not yet deposited to validators
    pub not_redeemed: Uint128, // amount of SIDE matured but not redeemed by user
    pub lsside_under_withdraw: Uint128, // amount of lsSIDE under 21 days withdraw
    pub side_under_withdraw: Uint128,
    pub lsside_to_burn: Uint128,
}
