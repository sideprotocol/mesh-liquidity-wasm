use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::ops::{Mul, Sub};

use crate::ContractError;
use crate::msg::ExecuteMsg;
use crate::state::STATE;
use crate::types::config::CONFIG;
use crate::types::validator_set::VALIDATOR_SET;
use crate::types::withdraw_window::{USER_CLAIMABLE, USER_CLAIMABLE_AMOUNT, ONGOING_WITHDRAWS_AMOUNT};
use crate::utils::{calc_threshold, calc_withdraw};
use crate::staking::{undelegate_msg, lsside_exchange_rate};
use crate::types::window_manager::{WINDOW_MANANGER, WindowManager};
use cosmwasm_std::{
    Env, StdError, Addr, StdResult,
    Uint128, Response, DepsMut, MessageInfo, CosmosMsg, WasmMsg, to_binary, Order,
};
use cw20::Cw20ExecuteMsg;

/**
 * Moves the current active window to validators and create new window as empty one.
 */
pub fn advance_window(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let mut messages = vec![];

    let config = CONFIG.load(deps.storage)?;

    if config.paused == true {
        return Err(ContractError::Std(StdError::generic_err(
            "The contract is temporarily paused",
        )));
    }

    if _info.sender != config.admin {
        return Err(ContractError::Std(StdError::generic_err(
            "Only admin can call advance window"
        )));
    }
    
    let mut state = STATE.load(deps.storage)?;
    let mut window_manager = WINDOW_MANANGER.load(deps.storage)?;

    let lsside_to_side;
    let mut claimed_juno = 0;
    let mut ongoing_side = 0;
    if check_window_advance(&env, &window_manager) {
        // trigger withdraw request on validator set
        let mut validator_set = VALIDATOR_SET.load(deps.storage)?;

        let exchange_rate_lsside = lsside_exchange_rate(deps.storage, deps.querier)?;
        let total_lsside_amount = window_manager.queue_window.total_lsside;
        lsside_to_side = calc_withdraw(total_lsside_amount, exchange_rate_lsside)?;

        state.side_under_withdraw += Uint128::from(lsside_to_side);
        state.lsside_under_withdraw += total_lsside_amount;

        // Backing update
        state.lsside_backing = Uint128::from(state.lsside_backing.u128().saturating_sub(lsside_to_side));

        let val_count = validator_set.validators.len();
        let total_staked = validator_set.total_staked();
        let mut left: u128 = 0;

        let mut remaining_rewards = Uint128::from(0u128);

        // only call Withdraw msg when lsside_to_side > 0
        if lsside_to_side > calc_threshold(total_staked, val_count) {
            // divide and withdraw from multiple validators
            if val_count == 0 {
                return Err(ContractError::Std(StdError::generic_err(
                    "No validator found!"
                )));
            }
            let to_withdraw = lsside_to_side.checked_div(val_count as u128).unwrap() - 2; // for division errors

            for validator in validator_set.clone().validators.iter() {
                let mut temp = to_withdraw + left;
                if validator.staked.u128() < temp {
                    left = temp - validator.staked.u128();
                    temp = validator.staked.u128();
                } else {
                    left = 0;
                }
                // reduce the amount from our stake tracker
                validator_set.unbond_from(&validator.address, temp)?;
                // debug_print!("Unbond {} from {}", temp, validator.address);
                // send the undelegate message
                messages.push(undelegate_msg(&validator.address, temp));
                // fetch the amount of rewards in the validator
                // and add it to backing and to_deposit
                let val_rewards = Uint128::from(
                    validator_set.query_rewards_validator(
                        deps.querier,
                        env.contract.address.to_string(),
                        validator.address.to_string()
                    )?
                );
                remaining_rewards += val_rewards;
            }
        } else if lsside_to_side > 0 {
            // withdraw from single validator with most staked amount
            // reduce the amount from our stake tracker
            let validator = validator_set.unbond_from_largest(lsside_to_side)?;
            // debug_print!("Unbond {} from {}", withdraw_amount_juno, validator);
            // send the undelegate message
            messages.push(undelegate_msg(&validator, lsside_to_side));
            // fetch the amount of rewards in the validator
            // and add it to backing and to_deposit
            let val_rewards = Uint128::from(
                validator_set.query_rewards_validator(
                    deps.querier,
                    env.contract.address.to_string(),
                    validator
                )?
            );
            remaining_rewards += val_rewards;
        }

        if remaining_rewards.u128() > 0 {
            state.to_deposit += remaining_rewards;
            state.lsside_backing += remaining_rewards;
        }

        validator_set.rebalance();
        VALIDATOR_SET.save(deps.storage, &validator_set)?;

        // BURN TOKENS
        let burn_msg_lsside = Cw20ExecuteMsg::Burn {
            amount: Uint128::from(total_lsside_amount)
        };

        // burn unbonding sejuno
        if total_lsside_amount != Uint128::from(0u128) {
            messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: config.ls_side_token.ok_or_else(|| {
                    ContractError::Std(StdError::generic_err(
                        "seJUNO token addr not registered".to_string(),
                    ))
                })?.to_string(),
                msg: to_binary(&burn_msg_lsside)?,
                funds: vec![],
            }));
        }

        // move active window object to ongoing_windows back
        window_manager.advance_window(
            deps.storage,
            env.block.time.seconds(),
            exchange_rate_lsside,
        )?;

        // mature part should be handled seperately
        // pop from front of ongoing_windows dequeue
        // while the mature timestamp of window is less than current time after active window append to back
        let mut ongoing_lsside = 0;
        let mut ongoing_users_side: HashMap<Addr, Uint128> = HashMap::new();
        while window_manager.ongoing_windows.len() as u128 > 0 {
            // continue only if front window is matured
            if let Some(front_window) = window_manager.ongoing_windows.front() {
                if front_window.time_to_mature_window >= env.block.time.seconds() {
                    break;
                }
            }

            // using prefix iteration
            let matured_window = window_manager.pop_matured(deps.storage)?;
            let side_window = matured_window.total_side;
            let lsside_window = matured_window.total_lsside;

            ongoing_side += side_window.u128();
            ongoing_lsside += lsside_window.u128();

            // store-optimize: Delete current window data
            // Add function to remove previous data
            let matured_amounts: StdResult<Vec<_>> = ONGOING_WITHDRAWS_AMOUNT
                .prefix(&matured_window.id.to_string())
                .range(deps.storage, None, None, Order::Ascending).collect();
            for (user_addr, user_juno_amount) in matured_amounts?.iter() {
                if let Some(already_stored_amount) = ongoing_users_side.get_mut(user_addr) {
                    *already_stored_amount += *user_juno_amount;
                } else {
                    ongoing_users_side.insert(user_addr.clone(), *user_juno_amount);
                }
                ONGOING_WITHDRAWS_AMOUNT.remove(deps.storage, (&matured_window.id.to_string(), user_addr),);
            }
        }

        if ongoing_side > 0 {
            let contract_juno = deps
                .querier
                .query_balance(env.contract.address.clone(), "ujuno")?
                .amount;
            claimed_juno =
                contract_juno.u128() - state.to_deposit.u128() - state.not_redeemed.u128();
            // If claimed is less than 90% of expected value, revert
            if claimed_juno < (ongoing_side * 90) / 100 {
                return Err(ContractError::Std(StdError::generic_err(
                    "Claim is not processed yet!"
                )));
            }

            state.side_under_withdraw = state.side_under_withdraw.sub(Uint128::from(ongoing_side)); // juno

            state.lsside_under_withdraw =
                state.lsside_under_withdraw.sub(Uint128::from(ongoing_lsside)); // lsside

            let mut ratio = Decimal::from(1u128);
            if ongoing_side > 0 {
                ratio = Decimal::from(claimed_juno as u64) / Decimal::from(ongoing_side as u64);
                if ratio > Decimal::from(1u128) {
                    ratio = Decimal::from(1u128);
                    state.not_redeemed += Uint128::from(ongoing_side);
                } else {
                    state.not_redeemed += Uint128::from(claimed_juno);
                }
            }

            if ongoing_users_side.len() as u128 > 0 {
                let mut user_claimable = USER_CLAIMABLE.load(deps.storage)?;
                for (user_addr, juno_amount) in ongoing_users_side.iter() {
                    let user_juno = (Decimal::from(juno_amount.u128() as u64).mul(ratio))
                        .to_u128()
                        .unwrap();
                    let mut after_user_juno = user_juno;
                    if let Some(current_user_juno) = USER_CLAIMABLE_AMOUNT.may_load(
                        deps.storage,
                        &user_addr,
                    )? {
                        after_user_juno += current_user_juno.u128();
                    }
                    USER_CLAIMABLE_AMOUNT.save(
                        deps.storage,
                        &user_addr,
                        &Uint128::from(after_user_juno),
                    )?;
                }
                user_claimable.total_side = Uint128::from(user_claimable.total_side.u128() + claimed_juno);
                USER_CLAIMABLE.save(deps.storage, &user_claimable)?;
            }
        }

        STATE.save(deps.storage, &state)?;
        WINDOW_MANANGER.save(deps.storage, &window_manager)?;
    } else {
        return Err(ContractError::Std(StdError::generic_err(
            "Advance window not available yet"
        )));
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "advance_window")
        .add_attribute("account", _info.sender.as_str())
        .add_attribute("withdraw_amount_side", lsside_to_side.to_string())
        .add_attribute("claimed_side", claimed_juno.to_string())
        .add_attribute("ongoing_side", ongoing_side.to_string()))
}

pub fn check_window_advance(env: &Env, window_manager: &WindowManager) -> bool {
    return window_manager.time_to_close_window <= env.block.time.seconds();
}