use cosmwasm_std::{
    DepsMut, Env, MessageInfo, Response, Uint128, CosmosMsg,
    WasmMsg, StdError, to_binary, StdResult, Storage, QuerierWrapper, CustomQuery, Addr
};
use cw20::{Cw20ReceiveMsg, Cw20ExecuteMsg};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use crate::staking::_calc_exchange_rate;
use crate::state::STATE;
use crate::ContractError;
use crate::tokens::query_total_supply;
use crate::types::config::CONFIG;

/**
 * Convert bJuno to seJuno.
 */
pub fn try_convert_to_sejuno(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let mut messages: Vec<CosmosMsg> = vec![];

    let config = CONFIG.load(deps.storage)?;
    let mut state = STATE.load(deps.storage)?;
    let sejuno_exch_rate = sejuno_exchange_rate_without_rewards(deps.storage, deps.querier)?;
    let bjuno_exch_rate = bjuno_exchange_rate_without_rewards(deps.storage, deps.querier)?;

    let sejuno_token = config.sejuno_token.ok_or_else(|| {
        ContractError::Std(StdError::generic_err(
            "seJuno token addr not registered".to_string(),
        ))
    })?.to_string();

    let bjuno_token = config.bjuno_token.ok_or_else(|| {
        ContractError::Std(StdError::generic_err(
            "bJuno token addr not registered".to_string(),
        ))
    })?.to_string();

    let mut bjuno_amount = _cw20_msg.amount.u128();

    // peg recovery fee
    let mut peg_fee = 0u128;
    let bjuno_threshold = Decimal::from(config.er_threshold)/Decimal::from(1000u64);
    let recovery_fee = Decimal::from(config.peg_recovery_fee)/Decimal::from(1000u64);
    if bjuno_exch_rate < bjuno_threshold {
        let max_peg_fee = recovery_fee.checked_mul(Decimal::from(bjuno_amount)).unwrap();
        let required_peg_fee =
            query_total_supply(deps.querier, &Addr::unchecked(bjuno_token.clone()))?.u128()
            .saturating_sub(state.bjuno_to_burn.u128() + state.bjuno_backing.u128());
        peg_fee = max_peg_fee.min(Decimal::from(required_peg_fee)).to_u128().unwrap();
        bjuno_amount = bjuno_amount.checked_sub(peg_fee).unwrap();
    }

    let juno_backing = bjuno_exch_rate.checked_mul(Decimal::from(bjuno_amount as u64))
                                        .unwrap().to_u128().unwrap();

    let sejuno_amount = Decimal::from(juno_backing as u64).checked_div(sejuno_exch_rate)
                                        .unwrap().to_u128().unwrap();

    // reduce bJuno backing
    state.bjuno_backing = state.bjuno_backing.saturating_sub(Uint128::from(juno_backing));

    // increase seJuno backing
    state.sejuno_backing += Uint128::from(juno_backing);

    state.bjuno_to_burn += Uint128::from(bjuno_amount+peg_fee); // this amount will be burned in ClaimAndStake

    // mint seJUNO to sender
    let mint_msg = Cw20ExecuteMsg::Mint {
        recipient: _cw20_msg.sender.to_string(),
        amount: sejuno_amount.into()
    };

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: sejuno_token,
        msg: to_binary(&mint_msg)?,
        funds: vec![],
    }));

    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("sejuno_minted", sejuno_amount.to_string())
        .add_attribute("juno_backing_value", juno_backing.to_string())
    )
}

/**
 * Convert seJuno to bJuno.
 */
pub fn try_convert_to_bjuno(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let mut messages: Vec<CosmosMsg> = vec![];

    let config = CONFIG.load(deps.storage)?;
    let mut state = STATE.load(deps.storage)?;
    let sejuno_exch_rate = sejuno_exchange_rate_without_rewards(deps.storage, deps.querier)?;
    let bjuno_exch_rate = bjuno_exchange_rate_without_rewards(deps.storage, deps.querier)?;

    let bjuno_token = config.bjuno_token.ok_or_else(|| {
        ContractError::Std(StdError::generic_err(
            "bJuno token addr not registered".to_string(),
        ))
    })?.to_string();

    let sejuno_amount = _cw20_msg.amount.u128();

    let juno_backing = sejuno_exch_rate.checked_mul(Decimal::from(sejuno_amount as u64))
                                        .unwrap().to_u128().unwrap();

    let mut bjuno_amount = Decimal::from(juno_backing as u64).checked_div(bjuno_exch_rate)
                                        .unwrap().to_u128().unwrap();

    // reduce seJuno backing
    state.sejuno_backing = state.sejuno_backing.saturating_sub(Uint128::from(juno_backing));

    // increase bJuno backing
    state.bjuno_backing += Uint128::from(juno_backing);

    state.sejuno_to_burn += Uint128::from(sejuno_amount); // this amount will be burned in ClaimAndStake

    // peg recovery fee
    let bjuno_threshold = Decimal::from(config.er_threshold)/Decimal::from(1000u64);
    let recovery_fee = Decimal::from(config.peg_recovery_fee)/Decimal::from(1000u64);
    if bjuno_exch_rate < bjuno_threshold {
        let max_peg_fee = recovery_fee.checked_mul(Decimal::from(bjuno_amount)).unwrap();
        let required_peg_fee =
            query_total_supply(deps.querier, &Addr::unchecked(bjuno_token.clone()))?.u128()
            .saturating_sub(state.bjuno_to_burn.u128() + state.bjuno_backing.u128());
        let peg_fee = max_peg_fee.min(Decimal::from(required_peg_fee)).to_u128().unwrap();
        bjuno_amount = bjuno_amount.checked_sub(peg_fee).unwrap();
    }

    // mint bJuno to sender
    let mint_msg = Cw20ExecuteMsg::Mint {
        recipient: _cw20_msg.sender.to_string(),
        amount: bjuno_amount.into()
    };

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: bjuno_token,
        msg: to_binary(&mint_msg)?,
        funds: vec![],
    }));

    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("bjuno_minted", bjuno_amount.to_string())
        .add_attribute("juno_backing_value", juno_backing.to_string())
    )
}

pub fn sejuno_exchange_rate_without_rewards<Q: CustomQuery>(
    storage: &dyn Storage,
    querier: QuerierWrapper<Q>,
) -> StdResult<Decimal> {
    let config = CONFIG.load(storage)?;
    let state = STATE.load(storage)?;
    let sejuno_token = config.sejuno_token.ok_or_else(|| {
        StdError::generic_err(
            "seJUNO token addr not registered".to_string(),
        )
    })?;

    let exch_rate = _calc_exchange_rate(
        state.sejuno_backing.u128(),
        query_total_supply(querier, &sejuno_token)?.u128()
            .saturating_sub(state.sejuno_to_burn.u128()),
    )?;

    Ok(exch_rate)
}

pub fn bjuno_exchange_rate_without_rewards<Q: CustomQuery>(
    storage: &dyn Storage,
    querier: QuerierWrapper<Q>,
) -> StdResult<Decimal> {
    let config = CONFIG.load(storage)?;
    let state = STATE.load(storage)?;
    let bjuno_token = config.bjuno_token.ok_or_else(|| {
        StdError::generic_err(
            "bJUNO token addr not registered".to_string(),
        )
    })?;

    let exch_rate = _calc_exchange_rate(
        state.bjuno_backing.u128(),
        query_total_supply(querier, &bjuno_token)?.u128()
            .saturating_sub(state.bjuno_to_burn.u128()),
    )?;

    Ok(exch_rate)
}