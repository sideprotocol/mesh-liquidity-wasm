use cosmwasm_std::{
    to_binary, Addr, BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response, StdError,
    Uint128, WasmMsg,
};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, ReferralMsg};
use std::convert::TryFrom;
use std::u128;

use cw20::Cw20ExecuteMsg;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

use crate::staking::{get_rewards, lsside_exchange_rate, stake_msg};
use crate::state::STATE;
use crate::types::config::CONFIG;
use crate::types::killswitch::KillSwitch;
use crate::types::validator_set::VALIDATOR_SET;
use crate::utils::calc_threshold;

const FEE_RESOLUTION: u128 = 100_000;

/**
 * Deposit SIDE amount to the contract and mint lsSIDE tokens using lsSIDE exchange rate.
 */
pub fn try_stake(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
    referral: u64,
) -> Result<Response, ContractError> {
    let mut messages: Vec<CosmosMsg> = vec![];

    let mut amount_raw: Uint128 = Uint128::default();
    let config = CONFIG.load(deps.storage)?;
    let mut state = STATE.load(deps.storage)?;

    if config.paused == true {
        return Err(ContractError::Std(StdError::generic_err(
            "The contract is temporarily paused",
        )));
    }

    let kill_switch = KillSwitch::try_from(config.kill_switch)?;

    if kill_switch == KillSwitch::Unbonding || kill_switch == KillSwitch::Open {
        return Err(ContractError::Std(StdError::generic_err(
            "Contract has been frozen. New deposits are not currently possible",
        )));
    }

    // read amount of SIDE sent by user on deposit
    for coin in &_info.funds {
        if coin.denom == "uside" {
            amount_raw = coin.amount
        }
    }

    if !config.referral_contract.is_none() && referral != 0 {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config
                .referral_contract
                .unwrap_or(Addr::unchecked(""))
                .to_string(),
            msg: to_binary(&ReferralMsg::Deposit {
                recipient: _info.sender.to_string(),
                code: referral,
                amount: amount_raw,
            })?,
            funds: vec![],
        }))
    }

    // if SIDE was not sent
    if amount_raw == Uint128::default() {
        return Err(ContractError::Std(StdError::generic_err(
            "Can only deposit a minimum of 1,000,000 uside (1 SIDE)".to_string(),
        )));
    }

    // if less than 1 SIDE was sent
    if amount_raw.u128() < 1_000_000 {
        return Err(ContractError::Std(StdError::generic_err(
            "Can only deposit a minimum of 1,000,000 uside (1 SIDE)".to_string(),
        )));
    }

    let lsside_token = config
        .ls_side_token
        .ok_or_else(|| {
            ContractError::Std(StdError::generic_err(
                "lsSIDE token addr not registered".to_string(),
            ))
        })?
        .to_string();

    // exch rate (SIDE staked + SIDE waiting withdraw) / (total supply in lsSIDE)
    let exch_rate = lsside_exchange_rate(deps.storage, deps.querier)?;

    // Update deposit amount
    state.to_deposit += amount_raw;
    state.lsside_backing += amount_raw;
    STATE.save(deps.storage, &state)?;
    // debug_print!("To deposit amount = {}", config.to_deposit);

    // Calculate amount of lsSIDE to be minted
    let token_amount = calc_deposit(amount_raw, exch_rate)?;

    let mint_msg = Cw20ExecuteMsg::Mint {
        recipient: _info.sender.to_string(),
        amount: token_amount.into(),
    };

    // mint message
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: lsside_token,
        msg: to_binary(&mint_msg)?,
        funds: vec![],
    }));

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "stake_for_lsside")
        .add_attribute("account", _info.sender.clone())
        .add_attribute("exch_rate_used", exch_rate.to_string())
        .add_attribute("lsSIDE amount", &token_amount.to_string()))
}

/**
 * Claim and stake amount to validators
 * Claim all outstanding rewards from validators and stake them.
 * If deposit + rewards amount is greater than threshold divide and
 * stake it into multiple validators,
 * else stake into validator with least stake
 */
pub fn try_claim_stake(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let mut messages: Vec<CosmosMsg> = vec![];

    let config = CONFIG.load(deps.storage)?;
    let mut state = STATE.load(deps.storage)?;
    let mut validator_set = VALIDATOR_SET.load(deps.storage)?;

    if config.paused == true {
        return Err(ContractError::Std(StdError::generic_err(
            "The contract is temporarily paused",
        )));
    }

    //TODO: check slashing on localnet
    let slashing_amount = (state.lsside_backing)
        .saturating_sub(state.to_deposit + Uint128::from(validator_set.total_staked()));
    state.lsside_backing = state
        .lsside_backing
        .saturating_sub(Uint128::from(slashing_amount.u128()));

    // claim rewards
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

    if state.lsside_to_burn.u128() > 0 {
        let burn_msg = Cw20ExecuteMsg::Burn {
            amount: state.lsside_to_burn,
        };

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
        state.lsside_to_burn = Uint128::from(0u128);
    }

    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "claim_and_stake")
        .add_attribute("rewards_claimed", reward_amount.to_string())
        .add_attribute("deposit amount", deposit_amount.to_string()))
}

/**
 * Calculates how much your deposited SIDE is worth in tokens
 * Adds the balance from the total supply and balance
 * Returns amount of tokens you get
 */
pub fn calc_deposit(amount: Uint128, exchange_rate: Decimal) -> Result<u128, ContractError> {
    let tokens_to_mint = Decimal::from(amount.u128() as u64)
        .checked_div(exchange_rate)
        .unwrap()
        .to_u128()
        .unwrap();

    Ok(tokens_to_mint)
}

/**
 * Calculates amount of fees from reward amount
 * and percentage of dev fees set in config.
 *
 * Returns amount of SIDE tokens in dev fees.
 */
pub fn calc_fee(amount: Uint128, fee: u64) -> u128 {
    amount
        .u128()
        .saturating_mul(fee as u128)
        .checked_div(FEE_RESOLUTION)
        .unwrap_or(0)
}
