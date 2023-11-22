use std::collections::VecDeque;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cw2::set_contract_version;
use cosmwasm_std::{
    Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, Uint128, VoteOption, CosmosMsg, 
    GovMsg
};

use crate::claim::claim;
use crate::deposit::{try_claim_stake, try_stake};
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, MigrateMsg};
use crate::query::{
    query_active_undelegation, query_current_window,
    query_dev_fee, query_info, query_pending_claims, query_sejuno_exchange_rate,
    query_user_claimable,
};
use crate::receive::try_receive_cw20;
use crate::staking::{redelegate_msg, get_onchain_balance_with_rewards};
use crate::tokens::query_total_supply;
use crate::state::{State, STATE, LSSIDE_FROZEN_TOTAL_ONCHAIN, LSSIDE_FROZEN_TOKENS};
use crate::admin::admin_commands;
use crate::types::config::{Config, CONFIG};
use crate::types::killswitch::KillSwitch;
use crate::types::validator_set::{VALIDATOR_SET, ValidatorSet};
use crate::types::window_manager::{WindowManager, WINDOW_MANANGER};
use crate::types::withdraw_window::{QueueWindow, UserClaimable, USER_CLAIMABLE};
use crate::update::{
    try_update_lsside_addr, rebalance_slash
};
use crate::window::advance_window;

// version info for migration info
const CONTRACT_NAME: &str = "stake-easy-staking-hub";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let config = Config {
        admin: info.sender.clone(),
        contract_addr: env.contract.address.clone(),
        ls_side_token: None,
        epoch_period: msg.epoch_period,
        unbonding_period: msg.unbonding_period,
        underlying_coin_denom: msg.underlying_coin_denom,
        reward_denom: msg.reward_denom,
        kill_switch: KillSwitch::Closed.into(),
        dev_fee: msg.dev_fee.unwrap_or(10000), // default: 10% of rewards
        dev_address: msg.dev_address,
        referral_contract: None,
        paused: false,
    };
    CONFIG.save(deps.storage, &config)?;

    // store state
    let state = State {
        lsside_backing: Uint128::from(0u128),
        to_deposit: Uint128::from(0u128), // amount of SIDE in contract (not validators)
        not_redeemed: Uint128::from(0u128),
        lsside_under_withdraw: Uint128::from(0u128), // amount of lsSIDE under 21 days withdraw
        side_under_withdraw: Uint128::from(0u128),
        lsside_to_burn: Uint128::from(0u128),
    };
    STATE.save(deps.storage, &state)?;

    let validator_set = ValidatorSet::default();
    VALIDATOR_SET.save(deps.storage, &validator_set)?;

    let default_manager = WindowManager {
        time_to_close_window: &env.block.time.seconds() + config.epoch_period,
        queue_window: QueueWindow {
            id: 0,
            total_lsside: Uint128::from(0u128),
        },
        ongoing_windows: VecDeque::new(),
    };
    WINDOW_MANANGER.save(deps.storage, &default_manager)?;

    let user_claimable = UserClaimable {
        total_side: Uint128::from(0u128),
    };
    USER_CLAIMABLE.save(deps.storage, &user_claimable)?;

    Ok(Response::new()
        .add_attribute("action", "init")
        .add_attribute("sender", info.sender.clone()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        //ADD one method to transfer all extra amount to dev address??
        ExecuteMsg::Stake { referral } => try_stake(deps, env, info, msg, referral),
        // claim rewards and stakes SIDE from contract to validators
        ExecuteMsg::ClaimAndStake {} => try_claim_stake(deps, env, info, msg),

        ExecuteMsg::UpdateLssideAddr { address } =>
        try_update_lsside_addr(deps, env, info, address),

        ExecuteMsg::Receive(_msg) => try_receive_cw20(deps, env, info, _msg),
        // init withdraw SIDE from validator to contract after 21 days and advance to next window
        ExecuteMsg::AdvanceWindow {} => advance_window(deps, env, info, msg),
        // for user to transfer SIDE from contract to self wallet if 21 day window is complete
        ExecuteMsg::Claim {} => claim(deps, env, info, msg),
        ExecuteMsg::VoteOnChain { proposal, vote } => try_vote(deps, env, info, proposal, vote),

        ExecuteMsg::RebalanceSlash {} => rebalance_slash(deps, env),
        ExecuteMsg::PauseContract {} => try_pause(deps, env, info),
        ExecuteMsg::UnpauseContract {} => try_unpause(deps, env, info),
        ExecuteMsg::Redelegate {from, to} => try_redelegate(deps, env, info, from, to),

        ExecuteMsg::RemoveValidator {address, redelegate} => try_remove_validator(deps, env, info, address, redelegate),
        ExecuteMsg::KillSwitchUnbond {} => try_kill_switch_unbond(deps, env, info),
        // extra admin commands
        _ => admin_commands(deps, env, info, msg),
    }
}

pub fn try_pause(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if _info.sender != config.admin {
        return Err(ContractError::Std(StdError::generic_err(
            "Pause can only be done from admin",
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
            "Unpause can only be done from admin",
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

    // Freeze lsside rate
    let lsside_token = config.ls_side_token.clone().ok_or_else(|| {
        StdError::generic_err(
            "lsSIDE token addr not registered".to_string(),
        )
    })?;
    let total_on_chain_se = get_onchain_balance_with_rewards(deps.querier, deps.storage, &contract_address,false)?;
    let tokens_se =
        query_total_supply(deps.querier, &lsside_token)?
        .saturating_sub(state.lsside_to_burn);
    let total_se = Uint128::from(total_on_chain_se);
    LSSIDE_FROZEN_TOTAL_ONCHAIN.save(deps.storage, &total_se)?;
    LSSIDE_FROZEN_TOKENS.save(deps.storage, &tokens_se)?;

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
            state.to_deposit += val_rewards;
            state.lsside_backing += val_rewards;
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
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Info {} => query_info(deps),
        QueryMsg::LssideExchangeRate {} => query_sejuno_exchange_rate(deps),
        QueryMsg::QueryDevFee {} => query_dev_fee(deps),
        QueryMsg::Window {} => query_current_window(deps),
        QueryMsg::Undelegations { address } => query_pending_claims(deps, address),
        QueryMsg::UserClaimable { address } => query_user_claimable(deps, address),
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
