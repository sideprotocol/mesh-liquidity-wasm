use std::collections::VecDeque;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cw2::set_contract_version;
use cosmwasm_std::{
    Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, Uint128, VoteOption, CosmosMsg, 
    GovMsg, WasmMsg, to_binary, Addr, Coin, BankMsg, Order
};
use cw20::Cw20ExecuteMsg;

use crate::claim::claim;
use crate::deposit::{try_claim_stake, try_stake, try_stake_for_bjuno};
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, RewardClaim, MigrateMsg};
use crate::query::{
    query_active_undelegation, query_bjuno_exchange_rate, query_current_window,
    query_dev_fee, query_info, query_pending_claims, query_sejuno_exchange_rate,
    query_top_validators, query_user_claimable,
};
use crate::deposit::calc_bjuno_reward;
use crate::receive::try_receive_cw20;
use crate::staking::{redelegate_msg, get_onchain_balance_with_rewards};
use crate::tokens::query_total_supply;
use crate::state::{State, STATE, BJUNO_FROZEN_TOTAL_ONCHAIN, BJUNO_FROZEN_TOKENS, SEJUNO_FROZEN_TOTAL_ONCHAIN, SEJUNO_FROZEN_TOKENS};
use crate::admin::admin_commands;
use crate::types::config::{Config, CONFIG};
use crate::types::killswitch::KillSwitch;
use crate::types::validator_set::{VALIDATOR_SET, ValidatorSet};
use crate::types::window_manager::{WindowManager, WINDOW_MANANGER};
use crate::types::withdraw_window::{QueueWindow, UserClaimable, USER_CLAIMABLE, USER_CLAIMABLE_AMOUNT, ONGOING_WITHDRAWS_AMOUNT, QUEUE_WINDOW_AMOUNT, BQUEUE_WINDOW_AMOUNT};
use crate::update::{
    try_update_bjuno_addr, try_update_rewards_addr, try_update_sejuno_addr,
    try_update_validator_set_addr, rebalance_slash
};
use crate::window::advance_window;
use crate::airdrop::{claim_airdrop, claim_airdrop_merkle_2, claim_airdrop_merkle_1};

// version info for migration info
const CONTRACT_NAME: &str = "stake-easy-staking-hub";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    if _msg.peg_recovery_fee.gt(&1000) {
        return Err(ContractError::Std(StdError::generic_err(
            "peg_recovery_fee can not be greater than 1000",
        )));
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let config = Config {
        admin: _info.sender.clone(),
        contract_addr: env.contract.address.clone(),
        sejuno_token: None,
        bjuno_token: None,
        top_validator_contract: None,
        rewards_contract: None,
        epoch_period: _msg.epoch_period,
        unbonding_period: _msg.unbonding_period,
        underlying_coin_denom: _msg.underlying_coin_denom,
        reward_denom: _msg.reward_denom,
        kill_switch: KillSwitch::Closed.into(),
        dev_fee: _msg.dev_fee.unwrap_or(10000), // default: 10% of rewards
        dev_address: _msg.dev_address,
        referral_contract: None,
        er_threshold: _msg.er_threshold.min(1000u64),
        peg_recovery_fee: _msg.peg_recovery_fee,
        paused: false,
    };
    CONFIG.save(deps.storage, &config)?;

    // store state
    let state = State {
        sejuno_backing: Uint128::from(0u128),
        bjuno_backing: Uint128::from(0u128),
        to_deposit: Uint128::from(0u128), // amount of JUNO in contract (not validators)
        not_redeemed: Uint128::from(0u128),
        bjuno_under_withdraw: Uint128::from(0u128), // amount of JUNO under 28 days withdraw
        sejuno_under_withdraw: Uint128::from(0u128), // amount of seJUNO under 28 days withdraw
        juno_under_withdraw: Uint128::from(0u128),
        sejuno_to_burn: Uint128::from(0u128),
        bjuno_to_burn: Uint128::from(0u128),
    };
    STATE.save(deps.storage, &state)?;

    let validator_set = ValidatorSet::default();
    VALIDATOR_SET.save(deps.storage, &validator_set)?;

    let default_manager = WindowManager {
        time_to_close_window: &env.block.time.seconds() + config.epoch_period,
        queue_window: QueueWindow {
            id: 0,
            total_sejuno: Uint128::from(0u128),
            total_bjuno: Uint128::from(0u128),
        },
        ongoing_windows: VecDeque::new(),
    };
    WINDOW_MANANGER.save(deps.storage, &default_manager)?;

    let user_claimable = UserClaimable {
        total_juno: Uint128::from(0u128),
    };
    USER_CLAIMABLE.save(deps.storage, &user_claimable)?;

    Ok(Response::new()
        .add_attribute("action", "init")
        .add_attribute("sender", _info.sender.clone()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match _msg {
        //ADD one method to transfer all extra amount to dev address??
        ExecuteMsg::Stake { referral } => try_stake(deps, env, _info, _msg, referral),
        ExecuteMsg::StakeForBjuno { referral } => try_stake_for_bjuno(deps, env, _info, _msg, referral),

        // claim rewards and stakes JUNO from contract to validators
        ExecuteMsg::ClaimAndStake {} => try_claim_stake(deps, env, _info, _msg),

        ExecuteMsg::UpdateSejunoAddr { address } =>
            try_update_sejuno_addr(deps, env, _info, address),
        ExecuteMsg::UpdateBjunoAddr { address } => 
            try_update_bjuno_addr(deps, env, _info, address),
        ExecuteMsg::UpdateValidatorSetAddr { address } =>
            try_update_validator_set_addr(deps, env, _info, address),
        ExecuteMsg::UpdateRewardsAddr { address } =>
            try_update_rewards_addr(deps, env, _info, address),

        ExecuteMsg::Receive(_msg) => try_receive_cw20(deps, env, _info, _msg),
        // init withdraw JUNO from validator to contract after 21 days and advance to next window
        ExecuteMsg::AdvanceWindow {} => advance_window(deps, env, _info, _msg),
        // for user to transfer JUNO from contract to self wallet if 21 day window is complete
        ExecuteMsg::Claim {} => claim(deps, env, _info, _msg),
        ExecuteMsg::VoteOnChain { proposal, vote } => try_vote(deps, env, _info, proposal, vote),

        ExecuteMsg::RebalanceSlash {} => rebalance_slash(deps, env),

        ExecuteMsg::ClaimAirdrop1 { address, stage, amount, proof } => 
         claim_airdrop_merkle_1(deps, env, _info, address, stage, amount, proof),

        ExecuteMsg::ClaimAirdrop2 { address, amount, proof } => 
         claim_airdrop_merkle_2(deps, env, _info, address, amount, proof),

        ExecuteMsg::ClaimAirdrop3 { address } => claim_airdrop(deps, env, _info, address),

        ExecuteMsg::PauseContract {} => try_pause(deps, env, _info),
        ExecuteMsg::UnpauseContract {} => try_unpause(deps, env, _info),
        ExecuteMsg::ClaimReward {} => try_claim(deps, env, _info),
        ExecuteMsg::Redelegate {from, to} => try_redelegate(deps, env, _info, from, to),

        ExecuteMsg::RemoveValidator {address, redelegate} => try_remove_validator(deps, env, _info, address, redelegate),
        ExecuteMsg::KillSwitchUnbond {} => try_kill_switch_unbond(deps, env, _info),

        ExecuteMsg::RemoveOldWindowData { window } => try_remove_data(deps, env, _info, window),
        ExecuteMsg::RemoveOldClaimData {} => try_remove_old_claim(deps, env, _info),
        ExecuteMsg::RemoveOldQueueData {} => try_remove_old_queue_data(deps, env, _info),

        // extra admin commands
        _ => admin_commands(deps, env, _info, _msg),
    }
}

pub fn try_remove_data(deps: DepsMut, _env: Env, info: MessageInfo, window: u64) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Std(StdError::generic_err(
            "Voting can only be done from voting admin",
        )));
    }

    let matured_amounts: StdResult<Vec<_>> = ONGOING_WITHDRAWS_AMOUNT
    .prefix(&window.to_string())
    .range(deps.storage, None, None, Order::Ascending).collect();

    // add mature window check, window should be matured
    // deleting only 0 values
    for (user_addr, user_juno_amount) in matured_amounts?.iter() {
        if *user_juno_amount == Uint128::from(0u128) {
            ONGOING_WITHDRAWS_AMOUNT.remove(
                deps.storage,
                (&window.to_string(), user_addr),
            );
        }
    }

    Ok(Response::new()
        .add_attribute("action", "Remove old window data with 0 balances"))
}

pub fn try_remove_old_claim(deps: DepsMut, _env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Std(StdError::generic_err(
            "Voting can only be done from voting admin",
        )));
    }

    let claim_amounts: StdResult<Vec<_>> = USER_CLAIMABLE_AMOUNT.range(deps.storage, None, None, Order::Ascending).collect();
    for (user_addr, user_juno_amount) in claim_amounts?.iter() {
        if *user_juno_amount == Uint128::from(0u128) {
            USER_CLAIMABLE_AMOUNT.remove(
                deps.storage,
                user_addr,
            );
        }
    }

    Ok(Response::new()
        .add_attribute("action", "Remove old claim 0 balances"))
}

pub fn try_remove_old_queue_data(deps: DepsMut, _env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Std(StdError::generic_err(
            "Voting can only be done from voting admin",
        )));
    }

    let queue_amounts: StdResult<Vec<_>> = QUEUE_WINDOW_AMOUNT.range(deps.storage, None, None, Order::Ascending).collect();
    let bqueue_amounts: StdResult<Vec<_>> = BQUEUE_WINDOW_AMOUNT.range(deps.storage, None, None, Order::Ascending).collect();

    for (user_addr, user_juno_amount) in queue_amounts?.iter() {
        if *user_juno_amount == Uint128::from(0u128) {
            QUEUE_WINDOW_AMOUNT.remove(
                deps.storage,
                user_addr,
            );
        }
    }

    for (user_addr, user_juno_amount) in bqueue_amounts?.iter() {
        if *user_juno_amount == Uint128::from(0u128) {
            BQUEUE_WINDOW_AMOUNT.remove(
                deps.storage,
                user_addr,
            );
        }
    }

    Ok(Response::new()
        .add_attribute("action", "Remove old active queue 0 balances"))
}

pub fn try_claim(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let reward_address = config.rewards_contract.ok_or_else(|| {
        ContractError::Std(StdError::generic_err(
            "reward addr not registered".to_string(),
        ))
    })?.to_string();
    let messages = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: reward_address,
        msg: to_binary(&RewardClaim::Claim { recipient: config.dev_address.to_string() })?,
        funds: vec![],
    })];

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("finalized claim", "Yes"))
}

pub fn try_pause(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if _info.sender != config.admin {
        return Err(ContractError::Std(StdError::generic_err(
            "Voting can only be done from voting admin",
        )));
    }

    config.paused = true;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new())
}

pub fn try_unpause(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if _info.sender != config.admin {
        return Err(ContractError::Std(StdError::generic_err(
            "Voting can only be done from voting admin",
        )));
    }

    config.paused = false;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new())
}


pub fn try_remove_validator(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    address: String,
    redelegate: Option<bool>
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if _info.sender != config.admin {
        return Err(ContractError::Std(StdError::generic_err(
            "Remove can only be done from admin",
        )));
    }

    let mut validator_set = VALIDATOR_SET.load(deps.storage)?;
    let mut messages: Vec<CosmosMsg> = vec![];
    let redelegate_flag = redelegate.unwrap_or(true);
    let removed = validator_set.remove(&address, redelegate_flag)?;

    // if redelegate flag is true, then move the redelegated amount
    // to validator with least staked amount in current set
    // Note: Remove will only work when redelegate flag will always be true,
    // with one exception when val.staked is zero.
    if let Some(validator) = removed {
        let to_stake = validator.staked;
        let dest_validator = validator_set.stake_with_least(to_stake.u128())?;
        if redelegate_flag {
            messages.push(redelegate_msg(&address, &dest_validator, to_stake.u128()));
        }
        validator_set.rebalance();
    }
    VALIDATOR_SET.save(deps.storage, &validator_set)?;
    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "Remove Validators"))
}

pub fn try_kill_switch_unbond(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    let state = STATE.load(deps.storage)?;
    let contract_address = _env.contract.address;
    if _info.sender != config.admin {
        return Err(ContractError::Std(StdError::generic_err(
            "kill can only be done from admin",
        )));
    }

    // Freeze bjuno rate
    let bjuno_token = config.bjuno_token.clone().ok_or_else(|| {
        StdError::generic_err(
            "bJUNO token addr not registered".to_string(),
        )
    })?;
    let total_on_chain = get_onchain_balance_with_rewards(deps.querier, deps.storage, &contract_address,true)?;
    let tokens =
        query_total_supply(deps.querier, &bjuno_token)?
        .saturating_sub(state.bjuno_to_burn);
    let total = Uint128::from(total_on_chain);
    BJUNO_FROZEN_TOTAL_ONCHAIN.save(deps.storage, &total)?;
    BJUNO_FROZEN_TOKENS.save(deps.storage, &tokens)?;

    // Freeze sejuno rate
    let sejuno_token = config.sejuno_token.clone().ok_or_else(|| {
        StdError::generic_err(
            "seJUNO token addr not registered".to_string(),
        )
    })?;
    let total_on_chain_se = get_onchain_balance_with_rewards(deps.querier, deps.storage, &contract_address,false)?;
    let tokens_se =
        query_total_supply(deps.querier, &sejuno_token)?
        .saturating_sub(state.sejuno_to_burn);
    let total_se = Uint128::from(total_on_chain_se);
    SEJUNO_FROZEN_TOTAL_ONCHAIN.save(deps.storage, &total_se)?;
    SEJUNO_FROZEN_TOKENS.save(deps.storage, &tokens_se)?;

    config.kill_switch = KillSwitch::Unbonding.into();
    CONFIG.save(deps.storage, &config)?;

    let mut validator_set = VALIDATOR_SET.load(deps.storage)?;

    let messages = validator_set.unbond_all();
    validator_set.zero();

    VALIDATOR_SET.save(deps.storage, &validator_set)?;

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "kill switch unbond"))
}

pub fn try_vote(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    proposal: u64,
    vote: VoteOption
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if config.paused == true {
        return Err(ContractError::Std(StdError::generic_err(
            "The contract is temporarily paused",
        )));
    }

    if _info.sender != config.admin {
        return Err(ContractError::Std(StdError::generic_err(
            "Voting can only be done from voting admin",
        )));
    }

    let messages = vec![CosmosMsg::Gov(GovMsg::Vote {
        proposal_id: proposal,
        vote,
    })];

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("finalized", "Yes"))
}

pub fn try_redelegate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    from: String,
    to: String
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // authenticate admin
    if config.admin != _info.sender {
        return Err(ContractError::Std(StdError::generic_err(
            "Only Admin can redelegate",
        )));
    }
    let mut validator_set = VALIDATOR_SET.load(deps.storage)?;
    let mut state = STATE.load(deps.storage)?;

    let mut messages: Vec<CosmosMsg> = vec![];
    let removed = validator_set.remove(&from, true)?;

    if let Some(validator) = removed {
        let to_stake = validator.staked;
        validator_set.stake_at(&to, to_stake.u128())?;

        let val_rewards = Uint128::from(
            validator_set.query_rewards_validator(
                deps.querier,
                env.contract.address.to_string(),
                from.clone()
            )?
        );
        if val_rewards.u128() > 0 {
            let bjuno_reward = calc_bjuno_reward(
                val_rewards,
                state.bjuno_backing,
                state.sejuno_backing
            )?;
            if bjuno_reward > 0 {
                let reward_contract_addr = if let Some(addr) = config.rewards_contract {
                    addr.to_string()
                }else{
                    return Err(ContractError::Std(StdError::generic_err(
                        "Reward contract is not registered",
                    )));
                };
                // Make different function for this due to call error
                messages.push(CosmosMsg::Bank(BankMsg::Send {
                    to_address: reward_contract_addr,
                    amount: vec![Coin {
                        denom: "ujuno".to_string(),
                        amount: Uint128::from(bjuno_reward),
                    }],
                }));
            }
            state.to_deposit += val_rewards.saturating_sub(Uint128::from(bjuno_reward));
            state.sejuno_backing += val_rewards.saturating_sub(Uint128::from(bjuno_reward));
        }

        messages.push(redelegate_msg(&from, &to, to_stake.u128()));
    }

    VALIDATOR_SET.save(deps.storage, &validator_set)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
    .add_messages(messages)
    .add_attribute("action", "Redelegate from one to other"))
}



#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    match _msg {
        QueryMsg::Info {} => query_info(deps),
        QueryMsg::SejunoExchangeRate {} => query_sejuno_exchange_rate(deps),
        QueryMsg::BjunoExchangeRate {} => query_bjuno_exchange_rate(deps),
        QueryMsg::QueryDevFee {} => query_dev_fee(deps),
        QueryMsg::Window {} => query_current_window(deps),
        QueryMsg::Undelegations { address } => query_pending_claims(deps, address),
        QueryMsg::UserClaimable { address } => query_user_claimable(deps, address),
        QueryMsg::ValidatorList {} => query_top_validators(deps),
        QueryMsg::ActiveUnbonding { address } => query_active_undelegation(deps, address),
    }
}

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let ver = cw2::get_contract_version(deps.storage)?;
    // ensure we are migrating from an allowed contract
    if ver.contract != CONTRACT_NAME {
        return Err(StdError::generic_err("Can only upgrade from same type").into());
    }
    // note: better to do proper semver compare, but string compare *usually* works
    if ver.version >= CONTRACT_VERSION.to_string() {
        return Err(StdError::generic_err("Cannot upgrade from a newer version").into());
    }

    // set the new version
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default())
}
