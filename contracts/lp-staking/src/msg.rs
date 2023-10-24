use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InstantiateMsg {
    pub deposit_token: String,
    pub reward_token: String,
    pub tokens_per_block: Uint128,
    pub total_alloc_point: Uint128,
    pub start_block: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ExecuteMsg {
    SetupPools {
        pools: Vec<(String, Uint128)>,
    },
    SetTokensPerBlock {
        /// The new amount of veToken to distribute per block
        amount: Uint128,
    },
    ClaimRewards {
        /// the LP token contract address
        lp_tokens: Vec<String>,
    },
    /// Withdraw LP tokens from contract
    Withdraw {
        /// The address of the LP token to withdraw
        lp_token: String,
        /// The amount to withdraw
        amount: Uint128,
    },
    UpdateConfig {},
    Receive(Cw20ReceiveMsg),
}

/// This structure describes custom hooks for the CW20.
#[derive(Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// Deposit performs a token deposit on behalf of the message sender.
    Deposit {},
    /// DepositFor performs a token deposit on behalf of another address that's not the message sender.
    DepositFor { beneficiary: Addr },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Returns total volume till latest timestamp
    TotalVolume {},
    /// Returns total volume till given timestamp
    TotalVolumeAt { timestamp: u64 },
    /// Returns contract address for which volume is tracked
    Contract {},
}
