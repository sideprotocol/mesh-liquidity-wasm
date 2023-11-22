use cosmwasm_std::{Addr, Uint128, VoteOption};
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{tokens::Contract, types::validator_set::ValidatorResponse};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub dev_address: Addr,
    pub dev_fee: Option<u64>,
    pub epoch_period: u64,
    pub underlying_coin_denom: String,
    pub unbonding_period: u64,
    pub reward_denom: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Stake {
        referral: u64,
    },
    Claim {},
    ClaimAndStake {},

    UpdateLssideAddr {
        address: Addr,
    },
    UpdateRewardsAddr {
        address: Addr,
    },

    // // token interaction
    Receive(Cw20ReceiveMsg),

    AdvanceWindow {},
    RebalanceSlash {},

    PauseContract {},
    UnpauseContract {},

    // voting
    VoteOnChain {
        proposal: u64,
        vote: VoteOption,
    },

    //Remove validator from set - redelegates all bonds to next available validator
    RemoveValidator {
        address: String,
        redelegate: Option<bool>,
    },

    // // add a new validator to the set
    //address in Addr(if string then will have to validate it before changing to Addr)
    AddValidator {
        address: Addr,
    },

    Redelegate {
        from: String,
        to: String,
    },

    ChangeOwner {
        new_owner: Addr,
    },
    // Unbond everything
    KillSwitchUnbond {},

    // // open the floodgates
    KillSwitchOpenWithdraws {},

    // TODO: Add tests for unbonding time.
    ChangeUnbondingTime {
        new_time: u64,
    },

    ChangeDevFee {
        dev_fee: Option<u64>,
        dev_address: Option<Addr>,
    },
    ChangeReferralContract {
        referral_contract: Addr,
    },
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AirdropMessage1 {
    Claim {
        stage: u8,
        amount: Uint128,
        proof: Vec<String>,
    },
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AirdropMessage2 {
    Claim { amount: Uint128, proof: Vec<String> },
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AirdropMessage3 {
    // TODO: add more variations here if found later
    Claim {},
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RewardClaim {
    Claim { recipient: String },
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReferralMsg {
    Deposit {
        recipient: String,
        code: u64,
        amount: Uint128,
    },
    Withdraw {
        recipient: String,
        amount: Uint128,
    },
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryRewards {
    AccruedRewards { address: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    LssideExchangeRate {},
    QueryDevFee {},
    Info {},
    Undelegations { address: Addr },
    UserClaimable { address: Addr },
    Window {},
    ActiveUnbonding { address: Addr },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AccruedRewardsResponse {
    pub rewards: Uint128,
}

// used by receive cw20
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    Unbond {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TopValidatorsResponse {
    pub validators: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsgValidator {
    GetValidators { top: i32, oth: i32, com: i32 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PendingClaimsResponse {
    pub window_id: u64,
    pub claim_time: u64,
    pub side_amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryResponse {
    Info {
        admin: Addr,
        validator_set: Vec<ValidatorResponse>,
        total_staked: Uint128,
        to_deposit: Uint128,
        lsside_backing: Uint128,
        lsside_to_burn: Uint128,
        lsside_in_contract: Uint128,
        side_under_withdraw: Uint128,
        ls_side_token: Addr,
        kill_switch: u8,
        epoch_period: u64,
        unbonding_period: u64,
        underlying_coin_denom: String,
        reward_denom: String,
        dev_address: Addr,
        dev_fee: u64,
    },
    PendingClaims {
        pending: Vec<PendingClaimsResponse>,
    },
    ActiveUndelegation {
        lsside_amount: Uint128,
    },
    TopValidators {
        validators: Vec<String>,
    },
    LssideExchangeRate {
        rate: String,
        denom: String,
    },
    DevFee {
        fee: u64,
        address: Addr,
    },
    Window {
        id: u64,
        time_to_close: u64,
        lsside_amount: Uint128,
    },
    Unbonding {
        unbonding_amount: Uint128,
    },
    Claimable {
        claimable_amount: Uint128,
    },
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TokenHandleMessage {
    SetVotingContract {
        contract: Option<Contract>,
        gov_token: bool,
    },
}
