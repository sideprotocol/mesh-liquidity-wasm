use cosmwasm_std::{Addr};
use cw_storage_plus::Item;
use serde::{Deserialize, Serialize};

pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Debug, Deserialize, Clone, PartialEq)]
pub struct Config {
    pub admin: Addr,
    pub contract_addr: Addr,
    pub sejuno_token: Option<Addr>,
    pub bjuno_token: Option<Addr>,
    pub top_validator_contract: Option<Addr>,
    pub rewards_contract: Option<Addr>,
    pub kill_switch: u8,
    pub epoch_period: u64,
    pub unbonding_period: u64,
    pub underlying_coin_denom: String,
    pub reward_denom: String,
    pub dev_address: Addr,
    pub dev_fee: u64,            // 10^-3 percent. 1 = 0.001%
    pub referral_contract: Option<Addr>,
    pub peg_recovery_fee: u64,
    pub er_threshold: u64,
    pub paused: bool,
}
