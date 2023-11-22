use std::cmp::min;
use std::convert::TryFrom;

use cosmwasm_std::{
    to_binary, Addr, BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response, StdError,
    Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use rust_decimal::prelude::ToPrimitive;

use crate::deposit::calc_fee;
use crate::msg::ReferralMsg;
use crate::staking::{get_balance, get_rewards, lsside_exchange_rate, stake_msg};
use crate::state::STATE;
use crate::types::config::CONFIG;
use crate::types::killswitch::KillSwitch;
use crate::types::validator_set::VALIDATOR_SET;
use crate::types::window_manager::WINDOW_MANANGER;
use crate::utils::{calc_threshold, calc_withdraw};
use crate::ContractError;

const MINIMUM_WITHDRAW: u128 = 10_000; // 0.01 seJUNO

/**
 * Adds user addr, amount to active withdraw window
 * bjuno is true when withdrawing juno for bjuno tokens.
 * bjuno is false when withdrawing juno for sejuno tokens.
 */
pub fn try_withdraw(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    _cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let mut messages: Vec<CosmosMsg> = vec![];

    let config = CONFIG.load(deps.storage)?;
    let mut state = STATE.load(deps.storage)?;
    let kill_switch = KillSwitch::try_from(config.kill_switch)?;

    if kill_switch == KillSwitch::Unbonding {
        return Err(ContractError::Std(StdError::generic_err(
            "Contract has been frozen.
                You must wait till unbonding has finished,
                then you will be able to withdraw your funds",
        )));
    }

    if kill_switch == KillSwitch::Open {
        return release_tokens(deps, env, _info, _cw20_msg);
    }

    // cannot withdraw less than 0.01 lsSIDE (10_000 lsSIDE without decimals)
    if _cw20_msg.amount < Uint128::from(MINIMUM_WITHDRAW) {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Amount withdrawn below minimum of {:?} ulsside",
            MINIMUM_WITHDRAW
        ))));
    }

    if !config.referral_contract.is_none() {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config
                .referral_contract
                .unwrap_or(Addr::unchecked(""))
                .to_string(),
            msg: to_binary(&ReferralMsg::Withdraw {
                recipient: _cw20_msg.sender.clone(),
                amount: _cw20_msg.amount,
            })?,
            funds: vec![],
        }))
    }

    let mut validator_set = VALIDATOR_SET.load(deps.storage)?;
    let mut window_manager = WINDOW_MANANGER.load(deps.storage)?;

    // Store user's lsside amount in active window (WithdrawWindow)
    window_manager.add_user_amount_to_active_window(
        deps.storage,
        Addr::unchecked(_cw20_msg.sender.clone()),
        _cw20_msg.amount,
    )?;
    let total_seside_amount = window_manager.queue_window.total_lsside;
    let user_seside_amount = window_manager.get_user_lsside_in_active_window(
        deps.storage,
        Addr::unchecked(_cw20_msg.sender.clone()),
    )?;

    WINDOW_MANANGER.save(deps.storage, &window_manager)?;

    messages.append(&mut validator_set.withdraw_rewards_messages());

    let reward_amount =
        get_rewards(deps.storage, deps.querier, &env.contract.address).unwrap_or_default();

    let fee = calc_fee(reward_amount, config.dev_fee);
    // Move fees to claim reward function only if fee > 0
    if (fee * 999 / 1000) > 0 {
        // leave a tiny amount in the contract for round error purposes
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: config.dev_address.to_string(),
            amount: vec![Coin {
                denom: "uside".to_string(),
                amount: Uint128::from(fee * 999 / 1000),
            }],
        }));
    }

    let total_reward_gen = Uint128::from(reward_amount.u128().saturating_sub(fee as u128));
    state.lsside_backing += total_reward_gen;

    let deposit_amount = state.to_deposit.u128() + total_reward_gen.u128();

    state.to_deposit = Uint128::from(0u128);
    STATE.save(deps.storage, &state)?;

    let val_count = validator_set.validators.len();
    let total_staked = validator_set.total_staked();

    // only call Delegate msg when deposit_amount > 0
    if deposit_amount > calc_threshold(total_staked, val_count) {
        // divide and deposit to multiple validators
        // check division
        if val_count == 0 {
            return Err(ContractError::Std(StdError::generic_err(
                "No validator found!",
            )));
        }
        let to_stake = deposit_amount.checked_div(val_count as u128).unwrap();
        let mut validator_idx: u128 = 0;

        for validator in validator_set.clone().validators.iter() {
            let mut to_stake_amt = to_stake;
            if validator_idx == val_count.clone().to_u128().unwrap() - 1 {
                to_stake_amt = deposit_amount.saturating_sub(
                    to_stake
                        .checked_mul(val_count.clone().to_u128().unwrap() - 1)
                        .unwrap(),
                );
            }
            validator_set.stake_at(&validator.address, to_stake_amt)?;
            // debug_print!("Staked {} at {}", to_stake_amt, validator.address);
            messages.push(stake_msg(&validator.address.clone(), to_stake_amt));
            validator_idx += 1;
        }
    } else if deposit_amount > 0 {
        // deposit to single validator with least staked amount
        // add the amount to our stake tracker
        let validator = validator_set.stake_with_least(deposit_amount)?;
        // send the stake message
        messages.push(stake_msg(&validator, deposit_amount));
    }

    validator_set.rebalance();
    VALIDATOR_SET.save(deps.storage, &validator_set)?;

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "withdraw")
        .add_attribute("total lsSIDE amount in active window", total_seside_amount)
        .add_attribute("user lsSIDE amount in active window", user_seside_amount))
}

/**
 * If bjuno is true then release amount for bjuno else
 * return amount for sejuno
 */
pub fn release_tokens(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    _cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let mut messages: Vec<CosmosMsg> = vec![];
    let config = CONFIG.load(deps.storage)?;

    let sejuno_xrate = lsside_exchange_rate(deps.storage, deps.querier)?;

    let side_amount = calc_withdraw(_cw20_msg.amount, sejuno_xrate)?;
    let my_balance = get_balance(deps.querier, &env.contract.address)?;

    let side_coin = Coin {
        denom: "uside".to_string(),
        amount: min(my_balance, Uint128::from(side_amount)),
    };

    let burn_msg = Cw20ExecuteMsg::Burn {
        amount: Uint128::from(_cw20_msg.amount),
    };

    // burn unbonding lsSIDE
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: config
            .ls_side_token
            .ok_or_else(|| {
                ContractError::Std(StdError::generic_err(
                    "lsSIDE token addr not registered".to_string(),
                ))
            })?
            .to_string(),
        msg: to_binary(&burn_msg)?,
        funds: vec![],
    }));

    messages.push(CosmosMsg::Bank(BankMsg::Send {
        to_address: _cw20_msg.sender.clone(),
        amount: vec![side_coin.clone()],
    }));

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "withdraw")
        .add_attribute("account", _cw20_msg.sender.clone())
        .add_attribute("amount", side_coin.amount))
}
