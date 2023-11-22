use cosmwasm_std::{
    Env, MessageInfo, DepsMut, Response, to_binary, BankMsg, Coin, CosmosMsg,
    StdError, Uint128, WasmMsg, Addr,
};

use std::convert::TryFrom;
use std::u128;
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, ReferralMsg};
use crate::tokens::query_total_supply;

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use cw20::Cw20ExecuteMsg;

use crate::staking::{get_rewards, stake_msg, sejuno_exchange_rate, bjuno_exchange_rate};
use crate::types::killswitch::KillSwitch;
use crate::types::validator_set::{VALIDATOR_SET};
use crate::utils::calc_threshold;
use crate::types::config::CONFIG;
use crate::state::{STATE,RewardExecuteMsg};

const FEE_RESOLUTION: u128 = 100_000;

/**
 * Deposit JUNO amount to the contract and mint seJUNO tokens using seJuno exchange rate.
 */
pub fn try_stake(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
    referral: u64
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

    // read amount of JUNO sent by user on deposit
    for coin in &_info.funds {
        if coin.denom == "ujuno" {
            amount_raw = coin.amount
        }
    }

    if !config.referral_contract.is_none() && referral != 0 {
        messages.push(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr:  config.referral_contract.unwrap_or(Addr::unchecked("")).to_string(),
                msg: to_binary(&ReferralMsg::Deposit { recipient: _info.sender.to_string(), code: referral, amount: amount_raw })?,
                funds: vec![],
            })
        )
    }

    // if JUNO was not sent
    if amount_raw == Uint128::default() {
        return Err(ContractError::Std(StdError::generic_err(
            "Can only deposit a minimum of 1,000,000 ujuno (1 JUNO)".to_string(),
        )));
    }

    // if less than 1 JUNO was sent
    if amount_raw.u128() < 1_000_000 {
        return Err(ContractError::Std(StdError::generic_err(
            "Can only deposit a minimum of 1,000,000 ujuno (1 JUNO)".to_string(),
        )));
    }

    let sejuno_token = config.sejuno_token.ok_or_else(|| {
        ContractError::Std(StdError::generic_err(
            "seJuno token addr not registered".to_string(),
        ))
    })?.to_string();

    // exch rate (JUNO staked + JUNO waiting withdraw) / (total supply in seJUNO)
    let exch_rate = sejuno_exchange_rate(deps.storage, deps.querier)?;

    // Update deposit amount
    state.to_deposit += amount_raw;
    state.sejuno_backing += amount_raw;
    STATE.save(deps.storage, &state)?;
    // debug_print!("To deposit amount = {}", config.to_deposit);

    // Calculate amount of seJUNO to be minted
    let token_amount = calc_deposit(amount_raw, exch_rate)?;

    let mint_msg = Cw20ExecuteMsg::Mint {
        recipient: _info.sender.to_string(),
        amount: token_amount.into()
    };

    // mint message
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: sejuno_token,
        msg: to_binary(&mint_msg)?,
        funds: vec![],
    }));

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "stake_for_sejuno")
        .add_attribute("account", _info.sender.clone())
        .add_attribute("exch_rate_used", exch_rate.to_string())
        .add_attribute("seJuno amount", &token_amount.to_string()))
}

/**
 * Deposit JUNO amount to the contract and mint bJUNO tokens using bJuno exchange rate.
 */
pub fn try_stake_for_bjuno(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
    referral: u64
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

    // read amount of JUNO sent by user on deposit
    for coin in &_info.funds {
        if coin.denom == "ujuno" {
            amount_raw = coin.amount
        }
    }

    if !config.referral_contract.is_none() && referral != 0 {
        messages.push(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr:  config.referral_contract.unwrap_or(Addr::unchecked("")).to_string(),
                msg: to_binary(&ReferralMsg::Deposit { recipient: _info.sender.to_string(), code: referral, amount: amount_raw })?,
                funds: vec![],
            })
        )
    }

    // if JUNO was not sent
    if amount_raw == Uint128::default() {
        return Err(ContractError::Std(StdError::generic_err(
            "Can only deposit a minimum of 1,000,000 ujuno (1 JUNO)".to_string(),
        )));
    }

    // if less than 1 JUNO was sent
    if amount_raw.u128() < 1_000_000 {
        return Err(ContractError::Std(StdError::generic_err(
            "Can only deposit a minimum of 1,000,000 ujuno (1 JUNO)".to_string(),
        )));
    }

    let bjuno_token = config.bjuno_token.ok_or_else(|| {
        ContractError::Std(StdError::generic_err(
            "bJuno token addr not registered".to_string(),
        ))
    })?.to_string();

    // exch rate (JUNO staked + JUNO waiting withdraw) / (total supply in bJuno)
    // TODO: read exch rate for bJuno from state stored
    let exch_rate = bjuno_exchange_rate(deps.storage, deps.querier)?;

    // Update deposit amount
    state.to_deposit += amount_raw;
    state.bjuno_backing += amount_raw;
    STATE.save(deps.storage, &state)?;
    // debug_print!("To deposit amount = {}", config.to_deposit);

    // Calculate amount of bJuno to be minted
    let mut token_amount = calc_deposit(amount_raw, exch_rate)?;

    // peg recovery fee
    let bjuno_threshold = Decimal::from(config.er_threshold)/Decimal::from(1000u64);
    let recovery_fee = Decimal::from(config.peg_recovery_fee)/Decimal::from(1000u64);
    if exch_rate < bjuno_threshold {
        let max_peg_fee = recovery_fee.checked_mul(Decimal::from(token_amount)).unwrap();
        let required_peg_fee =
            query_total_supply(deps.querier, &Addr::unchecked(bjuno_token.clone()))?.u128()
            .saturating_sub(state.bjuno_to_burn.u128() + state.bjuno_backing.u128());
        let peg_fee = max_peg_fee.min(Decimal::from(required_peg_fee)).to_u128().unwrap();
        token_amount = token_amount.checked_sub(peg_fee).unwrap();
    }

    let mint_msg = Cw20ExecuteMsg::Mint {
        recipient: _info.sender.to_string(),
        amount: token_amount.into()
    };

    // mint message
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: bjuno_token,
        msg: to_binary(&mint_msg)?,
        funds: vec![],
    }));

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "stake_for_bjuno")
        .add_attribute("account", _info.sender.clone())
        .add_attribute("exch_rate_used", exch_rate.to_string())
        .add_attribute("bJuno amount", &token_amount.to_string()))
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
    let slashing_amount = (state.sejuno_backing + state.bjuno_backing).saturating_sub(state.to_deposit + Uint128::from(validator_set.total_staked()));

    let bjuno_slashing_amount = calc_bjuno_reward(slashing_amount,state.bjuno_backing,state.sejuno_backing)?;

    state.bjuno_backing = state.bjuno_backing.saturating_sub(Uint128::from(bjuno_slashing_amount));
    state.sejuno_backing = state.sejuno_backing.saturating_sub(Uint128::from(slashing_amount.u128().saturating_sub(bjuno_slashing_amount)));

    // claim rewards
    messages.append(&mut validator_set.withdraw_rewards_messages());

    let reward_amount = get_rewards(deps.storage, deps.querier, &env.contract.address).unwrap_or_default();

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

    /*
     * LOGIC TO MOVE REWARD TO REWARD CONTRACT FOR BJUNO
     */
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

    state.to_deposit = Uint128::from(0u128);
    // debug_print!("To deposit amount = {}", config.to_deposit);

    let val_count = validator_set.validators.len();
    let total_staked = validator_set.total_staked();

    // only call Delegate msg when deposit_amount > 0
    if deposit_amount > calc_threshold(total_staked, val_count) {
        // divide and deposit to multiple validators
        // check division
        if val_count == 0 {
            return Err(ContractError::Std(StdError::generic_err("No validator found!")));
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

    // Burn converted bJUNO or seJUNO
    if state.bjuno_to_burn.u128() > 0 {
        let burn_msg = Cw20ExecuteMsg::Burn {
            amount: state.bjuno_to_burn
        };

        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.bjuno_token.ok_or_else(|| {
                ContractError::Std(StdError::generic_err(
                    "bJUNO token addr not registered".to_string(),
                ))
            })?.to_string(),
            msg: to_binary(&burn_msg)?,
            funds: vec![],
        }));
        state.bjuno_to_burn = Uint128::from(0u128);
    }
    if state.sejuno_to_burn.u128() > 0 {
        let burn_msg = Cw20ExecuteMsg::Burn {
            amount: state.sejuno_to_burn
        };

        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.sejuno_token.ok_or_else(|| {
                ContractError::Std(StdError::generic_err(
                    "seJUNO token addr not registered".to_string(),
                ))
            })?.to_string(),
            msg: to_binary(&burn_msg)?,
            funds: vec![],
        }));
        state.sejuno_to_burn = Uint128::from(0u128);
    }

    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "claim_and_stake")
        .add_attribute("rewards_claimed", reward_amount.to_string())
        .add_attribute("deposit amount", deposit_amount.to_string()))
}

/**
 * Calculates how much your deposited JUNO is worth in tokens
 * Adds the balance from the total supply and balance
 * Returns amount of tokens you get
 */
pub fn calc_deposit(
    amount: Uint128,
    exchange_rate: Decimal,
) -> Result<u128, ContractError> {
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
 * Returns amount of JUNO tokens in dev fees.
 */
pub fn calc_fee(amount: Uint128, fee: u64) -> u128 {
    amount
        .u128()
        .saturating_mul(fee as u128)
        .checked_div(FEE_RESOLUTION)
        .unwrap_or(0)
}
pub fn calc_bjuno_reward(
    total_reward: Uint128,
    bjuno_backing: Uint128,
    sejuno_backing:Uint128
) -> Result<u128, ContractError> {
    
    let total_juno = Decimal::from(bjuno_backing.u128() + sejuno_backing.u128());
    let bjuno_decimal = Decimal::from(bjuno_backing.u128());

    if (sejuno_backing + bjuno_backing) == Uint128::from(0u128) {
        return Ok((sejuno_backing + bjuno_backing).u128())
    }

    let bjuno_ratio = bjuno_decimal / total_juno;

    let bjuno_reward_amount = bjuno_ratio
    .checked_mul(Decimal::from(total_reward.u128()))
    .unwrap()
    .to_u128()
    .unwrap();

    Ok(bjuno_reward_amount)
}
