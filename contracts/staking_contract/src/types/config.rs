use cosmwasm_std::Addr;
use cw_storage_plus::Item;
use serde::{Deserialize, Serialize};

pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Debug, Deserialize, Clone, PartialEq)]
pub struct Config {
    pub admin: Addr,
    pub contract_addr: Addr,
    pub ls_side_token: Option<Addr>,
    pub kill_switch: u8,
    pub epoch_period: u64,
    pub unbonding_period: u64,
    pub underlying_coin_denom: String,
    pub reward_denom: String,
    pub dev_address: Addr,
    pub dev_fee: u64, // 10^-3 percent. 1 = 0.001%
    pub referral_contract: Option<Addr>,
    pub paused: bool,
}
