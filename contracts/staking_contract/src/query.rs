use cosmwasm_std::{
    Binary, StdResult, Deps, to_binary, Addr, Uint128, WasmQuery, QueryRequest, StdError, QuerierWrapper
};

use crate::msg::{QueryResponse, TopValidatorsResponse, QueryMsgValidator};
use crate::staking::{sejuno_exchange_rate, bjuno_exchange_rate, get_total_onchain_balance};
use crate::state::STATE;
use crate::types::config::CONFIG;
use crate::types::validator_set::{ValidatorSet, VALIDATOR_SET};
use crate::types::window_manager::WINDOW_MANANGER;
use crate::types::withdraw_window::{USER_CLAIMABLE_AMOUNT, QUEUE_WINDOW_AMOUNT, BQUEUE_WINDOW_AMOUNT};

pub fn query_info(deps: Deps) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;
    let state = STATE.load(deps.storage)?;
    let validator_set = VALIDATOR_SET.load(deps.storage)?;

    let total_on_chain = get_total_onchain_balance(deps.storage)?;

    to_binary(&QueryResponse::Info {
        admin: config.admin,
        validator_set: validator_set.to_query_response(),
        total_staked: Uint128::from(validator_set.total_staked()),
        to_deposit: Uint128::from(state.to_deposit),
        sejuno_backing: Uint128::from(state.sejuno_backing),
        bjuno_backing: Uint128::from(state.bjuno_backing),
        sejuno_to_burn: Uint128::from(state.sejuno_to_burn),
        bjuno_to_burn: Uint128::from(state.bjuno_to_burn),
        sejuno_in_contract: Uint128::from(state.sejuno_under_withdraw),
        bjuno_in_contract: Uint128::from(state.bjuno_under_withdraw),
        juno_under_withdraw: Uint128::from(state.juno_under_withdraw),
        sejuno_token: config.sejuno_token.unwrap_or(Addr::unchecked("")),
        bjuno_token: config.bjuno_token.unwrap_or(Addr::unchecked("")),
        top_validator_contract: config.top_validator_contract.unwrap_or(Addr::unchecked("")),
        rewards_contract: config.rewards_contract.unwrap_or(Addr::unchecked("")),
        epoch_period: config.epoch_period,
        unbonding_period: config.unbonding_period,
        underlying_coin_denom: config.underlying_coin_denom,
        reward_denom: config.reward_denom,
        dev_address: config.dev_address,
        dev_fee: config.dev_fee,
        kill_switch: config.kill_switch,
        er_threshold: config.er_threshold,
        peg_recovery_fee: config.peg_recovery_fee,
    })
}

pub fn query_dev_fee(deps: Deps) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;

    to_binary(&QueryResponse::DevFee {
        fee: config.dev_fee,
        address: config.dev_address,
    })
}

pub fn query_sejuno_exchange_rate(deps: Deps) -> StdResult<Binary> {
    let ratio = sejuno_exchange_rate(deps.storage, deps.querier)?;

    let rate = if ratio.is_zero() {
        "1".to_string()
    } else {
        ratio.to_string()
    };

    to_binary(&QueryResponse::SejunoExchangeRate {
        rate,
        denom: "ujuno".to_string(),
    })
}

pub fn query_bjuno_exchange_rate(deps: Deps) -> StdResult<Binary> {
    let ratio = bjuno_exchange_rate(deps.storage, deps.querier)?;

    let rate = if ratio.is_zero() {
        "1".to_string()
    } else {
        ratio.to_string()
    };

    to_binary(&QueryResponse::BjunoExchangeRate {
        rate,
        denom: "ujuno".to_string(),
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
        sejuno_amount: manager.queue_window.total_sejuno,
        bjuno_amount: manager.queue_window.total_bjuno,
    })
}

// query active undelegations
pub fn query_active_undelegation(_deps: Deps, address: Addr) -> StdResult<Binary> {
    let mut sejuno_amount = Uint128::from(0u128);
    let mut bjuno_amount = Uint128::from(0u128);

    if let Some(sejuno_value) = QUEUE_WINDOW_AMOUNT.may_load(_deps.storage, &address)? {
        sejuno_amount = sejuno_value.clone();
    }
    if let Some(bjuno_value) = BQUEUE_WINDOW_AMOUNT.may_load(_deps.storage, &address)? {
        bjuno_amount = bjuno_value.clone();
    }

    to_binary(&QueryResponse::ActiveUndelegation {
        sejuno_amount: sejuno_amount,
        bjuno_amount: bjuno_amount,
    })
}

pub fn query_user_claimable(deps: Deps, address: Addr) -> StdResult<Binary> {
    let mut user_juno_amount = Uint128::from(0u128);
    if let Some(user_claimable) = USER_CLAIMABLE_AMOUNT.may_load(deps.storage, &address)? {
        user_juno_amount = user_claimable;
    }

    to_binary(&QueryResponse::Claimable {
        claimable_amount: user_juno_amount,
    })
}

pub fn query_delegation(
    querier: &QuerierWrapper,
    validator: &str,
    contract_addr: &Addr
 ) -> StdResult<u128> {
    Ok(querier.query_delegation(contract_addr, validator)?.map(|fd| fd.amount.amount.u128()).unwrap_or(0))
}

pub fn query_top_validators(deps: Deps) -> StdResult<Binary> {
    let validators_list = get_top_validators(deps)?;

    to_binary(&QueryResponse::TopValidators {
        validators: validators_list.validators,
    })
}

pub fn fetch_validator_set_from_contract(deps: Deps) -> StdResult<ValidatorSet> {
    let network_vals = deps.querier.query_all_validators()?;

    let validators_list = get_top_validators(deps)?;

    let mut validator_set = ValidatorSet::default();
    for validator_addr in validators_list.validators.iter() {
        // ensure the validator is registered
        if !network_vals
            .iter()
            .any(|v| v.address == Addr::unchecked(validator_addr.clone()))
        {
            return Err(StdError::generic_err(format!(
                "{} is not in the current validator set",
                validator_addr
            )));
        }
        validator_set.add(validator_addr.clone());
    }

    Ok(validator_set)
}

pub fn get_top_validators(deps: Deps) -> StdResult<TopValidatorsResponse> {
    let config = CONFIG.load(deps.storage)?;

    let validators_list: TopValidatorsResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: config.top_validator_contract.ok_or_else(|| {
                StdError::generic_err(
                    "top validator contract addr not registered".to_string(),
                )
            })?.to_string(),
            msg: to_binary(&QueryMsgValidator::GetValidators {
                top: 2,
                oth: 1,
                com: 2,
            })?,
        }))?;

    Ok(validators_list)
}
