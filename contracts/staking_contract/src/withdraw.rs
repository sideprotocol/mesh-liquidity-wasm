use std::cmp::min;
use std::convert::TryFrom;

use cosmwasm_std::{
    BankMsg, Coin, CosmosMsg, Env, Response, Addr,
    StdError, Uint128, DepsMut, MessageInfo, WasmMsg, to_binary
};
use cw20::{Cw20ReceiveMsg, Cw20ExecuteMsg};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use crate::ContractError;
use crate::msg::ReferralMsg;
use crate::deposit::{calc_fee, calc_bjuno_reward};
use crate::staking::{get_rewards, stake_msg, sejuno_exchange_rate, bjuno_exchange_rate, get_balance};
use crate::state::STATE;
use crate::tokens::query_total_supply;
use crate::types::config::CONFIG;
use crate::types::killswitch::KillSwitch;
use crate::types::validator_set::VALIDATOR_SET;
use crate::types::window_manager::WINDOW_MANANGER;
use crate::utils::{calc_threshold, calc_withdraw};

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
    bjuno: bool,
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
        return release_tokens(deps, env, _info, _cw20_msg, bjuno);
    }

    // cannot withdraw less than 0.01 seJUNO or bJUNO (10_000 seJUNO or bJUNO without decimals)
    if _cw20_msg.amount < Uint128::from(MINIMUM_WITHDRAW) {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Amount withdrawn below minimum of {:?} usejuno or ubjuno",
            MINIMUM_WITHDRAW
        ))));
    }

    if !config.referral_contract.is_none() {
        messages.push(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr:  config.referral_contract.unwrap_or(Addr::unchecked("")).to_string(),
                msg: to_binary(&ReferralMsg::Withdraw { recipient: _cw20_msg.sender.clone(), bjuno: bjuno, amount: _cw20_msg.amount })?,
                funds: vec![],
            })
        )
    }

    let mut validator_set = VALIDATOR_SET.load(deps.storage)?;
    let mut window_manager = WINDOW_MANANGER.load(deps.storage)?;

    let mut total_sejuno_amount = window_manager.queue_window.total_sejuno;
    let mut user_sejuno_amount = window_manager.get_user_sejuno_in_active_window(
        deps.storage,
        Addr::unchecked(_cw20_msg.sender.clone()),
    )?;
    let mut total_bjuno_amount = window_manager.queue_window.total_bjuno;
    let mut user_bjuno_amount = window_manager.get_user_bjuno_in_active_window(
        deps.storage,
        Addr::unchecked(_cw20_msg.sender.clone()),
    )?;

    let mut peg_fee = 0u128;

    // Store user's seJUNO or bJUNO amount in active window (WithdrawWindow)
    if !bjuno { // seJUNO sent by user
        window_manager.add_user_amount_to_active_window(
            deps.storage,
            Addr::unchecked(_cw20_msg.sender.clone()),
            _cw20_msg.amount,
            Uint128::from(0u128)
        )?;
        total_sejuno_amount = window_manager.queue_window.total_sejuno;
        user_sejuno_amount = window_manager.get_user_sejuno_in_active_window(
            deps.storage,
            Addr::unchecked(_cw20_msg.sender.clone()),
        )?;
    }
    else { // bJUNO sent by user
        let bjuno_exch_rate = bjuno_exchange_rate(deps.storage, deps.querier)?;
        let mut bjuno_amount = _cw20_msg.amount.clone().u128();
        let bjuno_token = config.bjuno_token.ok_or_else(|| {
            ContractError::Std(StdError::generic_err(
                "bJuno token addr not registered".to_string(),
            ))
        })?.to_string();

        // peg recovery fee
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

        window_manager.add_user_amount_to_active_window(
            deps.storage,
            Addr::unchecked(_cw20_msg.sender.clone()),
            Uint128::from(0u128),
            Uint128::from(bjuno_amount),
        )?;
        total_bjuno_amount = window_manager.queue_window.total_bjuno;
        user_bjuno_amount = window_manager.get_user_bjuno_in_active_window(
            deps.storage,
            Addr::unchecked(_cw20_msg.sender.clone()),
        )?;
    }

    if peg_fee > 0 {
        state.bjuno_to_burn += Uint128::from(peg_fee);
    }

    WINDOW_MANANGER.save(deps.storage, &window_manager)?;

    messages.append(&mut validator_set.withdraw_rewards_messages());

    let reward_amount = get_rewards(
        deps.storage,
        deps.querier,
        &env.contract.address
    ).unwrap_or_default();

    let fee = calc_fee(reward_amount, config.dev_fee);
    // Move fees to claim reward function only if fee > 0
    if (fee * 999 / 1000) > 0 {  // leave a tiny amount in the contract for round error purposes
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: config.dev_address.to_string(),
            amount: vec![Coin {
                denom: "ujuno".to_string(),
                amount: Uint128::from(fee * 999 / 1000),
            }],
        }));
    }

    let total_reward_gen = Uint128::from(reward_amount.u128().saturating_sub(fee as u128));

    let reward_contract_addr = if let Some(addr) = config.rewards_contract {
        addr.to_string()
    }else{
        return Err(ContractError::Std(StdError::generic_err(
            "Reward contract is not registered",
        )));
    };

    let bjuno_reward = calc_bjuno_reward(total_reward_gen,state.bjuno_backing,state.sejuno_backing)?;

    if bjuno_reward > 0 {
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: reward_contract_addr.clone(),
            amount: vec![Coin {
                denom: "ujuno".to_string(),
                amount: Uint128::from(bjuno_reward),
            }],
        }));
    }

    let global_idx_update_msg = RewardExecuteMsg::UpdateGlobalIndex{};
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: reward_contract_addr.clone(),
        msg: to_binary(&global_idx_update_msg)?,
        funds: vec![],
    }));

    let reward_to_add = total_reward_gen.checked_sub(Uint128::from(bjuno_reward)).unwrap();

    state.sejuno_backing += reward_to_add;

    let deposit_amount = state.to_deposit.u128() + reward_to_add.u128();
    // let deposit_amount = state.to_deposit.u128() + reward_amount_to_add.u128();

    state.to_deposit = Uint128::from(0u128);
    STATE.save(deps.storage, &state)?;
    // debug_print!("To deposit amount = {}", state.to_deposit);

    let val_count = validator_set.validators.len();
    let total_staked = validator_set.total_staked();

    // only call Delegate msg when deposit_amount > 0
    if deposit_amount > calc_threshold(total_staked, val_count) {
        // divide and deposit to multiple validators
        // check division
        if val_count == 0 {
            return Err(ContractError::Std(StdError::generic_err(
                "No validator found!"
            )));
        }
        let to_stake = deposit_amount.checked_div(val_count as u128).unwrap();
        let mut validator_idx: u128 = 0;

        for validator in validator_set.clone().validators.iter() {
            let mut to_stake_amt = to_stake;
            if validator_idx == val_count.clone().to_u128().unwrap()-1 {
                to_stake_amt = deposit_amount.saturating_sub(
                    to_stake.checked_mul(val_count.clone().to_u128().unwrap()-1).unwrap()
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
        .add_attribute("total seJUNO amount in active window", total_sejuno_amount)
        .add_attribute("total bJUNO amount in active window", total_bjuno_amount)
        .add_attribute("user seJUNO amount in active window", user_sejuno_amount)
        .add_attribute("user bJUNO amount in active window", user_bjuno_amount))
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
    bjuno: bool
) -> Result<Response, ContractError> {
    let mut messages: Vec<CosmosMsg> = vec![];
    let config = CONFIG.load(deps.storage)?;

    // debug_print(format!("** tokens withdrawn: {}", amount));
    let sejuno_xrate = sejuno_exchange_rate(deps.storage,deps.querier)?;
    let bjuno_xrate = bjuno_exchange_rate(deps.storage,deps.querier)?;

    // debug_print(format!("** Frozen sejuno exchange rate: {}", sejuno_xrate.to_string()));
    // debug_print(format!("** Frozen bjuno exchange rate: {}", bjuno_xrate.to_string()));
    let mut juno_amount:u128 = 0;
    if bjuno {
        juno_amount = calc_withdraw(_cw20_msg.amount, bjuno_xrate)?;
    }else{
        juno_amount = calc_withdraw(_cw20_msg.amount, sejuno_xrate)?;
    }
    // debug_print(format!("** JUNO amount withdrawn: {}", juno_amount));
    let my_balance = get_balance(deps.querier, &env.contract.address)?;
    // debug_print(format!("** contract balance: {}", my_balance));

    let juno_coin = Coin {
        denom: "ujuno".to_string(),
        amount: min(my_balance, Uint128::from(juno_amount)),
    };

    let burn_msg = Cw20ExecuteMsg::Burn {
        amount: Uint128::from(_cw20_msg.amount)
    };

    // burn unbonding lsSIDE
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: config.ls_side_token.ok_or_else(|| {
            ContractError::Std(StdError::generic_err(
                "lsSIDE token addr not registered".to_string(),
            ))
        })?.to_string(),
        msg: to_binary(&burn_msg)?,
        funds: vec![],
    }));

    messages.push(CosmosMsg::Bank(BankMsg::Send {
        to_address: _cw20_msg.sender.clone(),
        amount: vec![juno_coin.clone()],
    }));

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "withdraw")
        .add_attribute("account", _cw20_msg.sender.clone())
        .add_attribute("amount", juno_coin.amount))
}
