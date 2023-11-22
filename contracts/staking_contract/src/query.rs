use cosmwasm_std::{to_binary, Addr, Binary, Deps, QuerierWrapper, StdResult, Uint128};

use crate::msg::QueryResponse;
use crate::staking::lsside_exchange_rate;
use crate::state::STATE;
use crate::types::config::CONFIG;
use crate::types::validator_set::VALIDATOR_SET;
use crate::types::window_manager::WINDOW_MANANGER;
use crate::types::withdraw_window::{QUEUE_WINDOW_AMOUNT, USER_CLAIMABLE_AMOUNT};

pub fn query_info(deps: Deps) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;
    let state = STATE.load(deps.storage)?;
    let validator_set = VALIDATOR_SET.load(deps.storage)?;

    to_binary(&QueryResponse::Info {
        admin: config.admin,
        validator_set: validator_set.to_query_response(),
        total_staked: Uint128::from(validator_set.total_staked()),
        to_deposit: Uint128::from(state.to_deposit),
        lsside_backing: Uint128::from(state.lsside_backing),
        lsside_to_burn: Uint128::from(state.lsside_to_burn),
        lsside_in_contract: Uint128::from(state.lsside_under_withdraw),
        side_under_withdraw: Uint128::from(state.side_under_withdraw),
        ls_side_token: config.ls_side_token.unwrap_or(Addr::unchecked("")),
        epoch_period: config.epoch_period,
        unbonding_period: config.unbonding_period,
        underlying_coin_denom: config.underlying_coin_denom,
        reward_denom: config.reward_denom,
        dev_address: config.dev_address,
        dev_fee: config.dev_fee,
        kill_switch: config.kill_switch,
    })
}

pub fn query_dev_fee(deps: Deps) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;

    to_binary(&QueryResponse::DevFee {
        fee: config.dev_fee,
        address: config.dev_address,
    })
}

pub fn query_lsside_exchange_rate(deps: Deps) -> StdResult<Binary> {
    let ratio = lsside_exchange_rate(deps.storage, deps.querier)?;

    let rate = if ratio.is_zero() {
        "1".to_string()
    } else {
        ratio.to_string()
    };

    to_binary(&QueryResponse::LssideExchangeRate {
        rate,
        denom: "uside".to_string(),
    })
}

pub fn query_pending_claims(deps: Deps, address: Addr) -> StdResult<Binary> {
    let window_manager = WINDOW_MANANGER.load(deps.storage)?;
    let pending_withdraws = window_manager.get_user_pending_withdraws(deps.storage, address)?;

    to_binary(&QueryResponse::PendingClaims {
        pending: pending_withdraws,
    })
}

pub fn query_current_window(deps: Deps) -> StdResult<Binary> {
    let manager = WINDOW_MANANGER.load(deps.storage)?;

    to_binary(&QueryResponse::Window {
        id: manager.queue_window.id,
        time_to_close: manager.time_to_close_window,
        lsside_amount: manager.queue_window.total_lsside,
    })
}

// query active undelegations
pub fn query_active_undelegation(_deps: Deps, address: Addr) -> StdResult<Binary> {
    let mut lsside_amount = Uint128::from(0u128);

    if let Some(lsside_value) = QUEUE_WINDOW_AMOUNT.may_load(_deps.storage, &address)? {
        lsside_amount = lsside_value.clone();
    }

    to_binary(&QueryResponse::ActiveUndelegation {
        lsside_amount: lsside_amount,
    })
}

pub fn query_user_claimable(deps: Deps, address: Addr) -> StdResult<Binary> {
    let mut user_side_amount = Uint128::from(0u128);
    if let Some(user_claimable) = USER_CLAIMABLE_AMOUNT.may_load(deps.storage, &address)? {
        user_side_amount = user_claimable;
    }

    to_binary(&QueryResponse::Claimable {
        claimable_amount: user_side_amount,
    })
}

pub fn query_delegation(
    querier: &QuerierWrapper,
    validator: &str,
    contract_addr: &Addr,
) -> StdResult<u128> {
    Ok(querier
        .query_delegation(contract_addr, validator)?
        .map(|fd| fd.amount.amount.u128())
        .unwrap_or(0))
}
