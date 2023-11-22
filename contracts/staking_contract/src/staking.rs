use std::convert::TryFrom;
use std::u128;

use cosmwasm_std::{DistributionMsg, QuerierWrapper, CustomQuery};
use cosmwasm_std::{
    Coin, CosmosMsg, Addr, StakingMsg,
    StdError, StdResult, Storage, Uint128, DepsMut, Env, MessageInfo, Response
};
use crate::ContractError;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;

use crate::deposit::{calc_fee, calc_bjuno_reward};
use crate::state::{STATE, SEJUNO_FROZEN_TOTAL_ONCHAIN, BJUNO_FROZEN_TOTAL_ONCHAIN, SEJUNO_FROZEN_TOKENS, BJUNO_FROZEN_TOKENS};
use crate::tokens::query_total_supply;
use crate::types::config::CONFIG;
use crate::types::killswitch::KillSwitch;
use crate::types::validator_set::{VALIDATOR_SET};

pub fn sejuno_exchange_rate<Q: CustomQuery>(
    store: &dyn Storage,
    querier: QuerierWrapper<Q>,
) -> StdResult<Decimal> {
    let config = CONFIG.load(store)?;
    let state = STATE.load(store)?;
    let contract_address = config.contract_addr;

    let sejuno_token = config.sejuno_token.ok_or_else(|| {
        StdError::generic_err(
            "seJUNO token addr not registered".to_string(),
        )
    })?;

    if KillSwitch::try_from(config.kill_switch)? == KillSwitch::Closed {
        let total_on_chain = get_onchain_balance_with_rewards(querier, store, &contract_address,false)?;
        let tokens =
            query_total_supply(querier, &sejuno_token)?.u128()
            .saturating_sub(state.sejuno_to_burn.u128());
        let exchange_rate = _calc_exchange_rate(total_on_chain, tokens)?;
        Ok(exchange_rate)
    } else {
        let total_on_chain = SEJUNO_FROZEN_TOTAL_ONCHAIN.load(store)?.u128();
        let tokens = SEJUNO_FROZEN_TOKENS.load(store)?.u128();
    
        let exchange_rate = _calc_exchange_rate(total_on_chain, tokens)?;
        Ok(exchange_rate)
    }
}

pub fn bjuno_exchange_rate<Q: CustomQuery>(
    store: &dyn Storage,
    querier: QuerierWrapper<Q>,
) -> StdResult<Decimal> {
    let config = CONFIG.load(store)?;
    let state = STATE.load(store)?;
    let contract_address = config.contract_addr;

    let bjuno_token = config.bjuno_token.ok_or_else(|| {
        StdError::generic_err(
            "bJUNO token addr not registered".to_string(),
        )
    })?;

    if KillSwitch::try_from(config.kill_switch)? == KillSwitch::Closed {
        let total_on_chain = get_onchain_balance_with_rewards(querier, store, &contract_address, true)?;
        let tokens =
            query_total_supply(querier, &bjuno_token)?.u128()
            .saturating_sub(state.bjuno_to_burn.u128());
        let exchange_rate = _calc_exchange_rate(total_on_chain, tokens)?;

        Ok(exchange_rate)
    } else {
        let total_on_chain = BJUNO_FROZEN_TOTAL_ONCHAIN.load(store)?.u128();
        let tokens = BJUNO_FROZEN_TOKENS.load(store)?.u128();
 
        let exchange_rate = _calc_exchange_rate(total_on_chain, tokens)?;
        Ok(exchange_rate)
    }
}

pub fn _calc_exchange_rate(
    total_on_chain: u128,
    tokens: u128
) -> StdResult<Decimal> {
    let juno_balance = Decimal::from(total_on_chain as u64);
    let token_bal = Decimal::from(tokens as u64);

    let ratio = if total_on_chain == 0 || tokens == 0 {
        Decimal::one()
    } else {
        juno_balance.checked_div(token_bal).unwrap()
    };

    Ok(ratio.round_dp_with_strategy(12, RoundingStrategy::AwayFromZero))
}

//return net_onchain balance with rewards for bjuno and sejuno using backing to calculate amount
// check if we can also update backing here if total_staked differ with backing data
// when bjuno is true => returns bjuno onchain balance else
//                       returns sejuno onchain balance
pub fn get_onchain_balance_with_rewards<Q: CustomQuery>(
    querier: QuerierWrapper<Q>,
    storage: &dyn Storage,
    contract_address: &Addr,
    bjuno: bool,
) -> StdResult<u128> {
    let config = CONFIG.load(storage)?;
    let state = STATE.load(storage)?;

    let rewards_balance = get_rewards(storage, querier, contract_address)
        .unwrap_or_default();

    let fee = calc_fee(rewards_balance, config.dev_fee);
    let final_rewards = Uint128::from(rewards_balance.u128().saturating_sub(fee as u128));
    let bjuno_reward_bal = calc_bjuno_reward(final_rewards,state.bjuno_backing,state.sejuno_backing).unwrap();

    let bjuno_onchain_balance = state.bjuno_backing;
    let sejuno_onchain_balance = state.sejuno_backing + Uint128::from(final_rewards.u128()-bjuno_reward_bal);
    if bjuno {
        Ok(bjuno_onchain_balance.u128())
    }else{
        Ok(sejuno_onchain_balance.u128())
    }
}

pub fn get_total_onchain_balance(
    storage: &dyn Storage,
) -> StdResult<u128> {
    let validator_set = VALIDATOR_SET.load(storage)?;
    let locked_balance = validator_set.total_staked();

    let state = STATE.load(storage)?;
    let to_deposit_balance = state.to_deposit.u128();

    Ok(locked_balance+to_deposit_balance)
}

pub fn get_balance<Q: CustomQuery>(
    querier: QuerierWrapper<Q>,
    address: &Addr,
) -> StdResult<Uint128> {
    let balance = querier.query_balance(address.clone(), &"ujuno".to_string())?;

    Ok(balance.amount)
}

pub fn get_rewards<Q: CustomQuery>(
    storage: &dyn Storage,
    querier: QuerierWrapper<Q>,
    contract: &Addr,
) -> StdResult<Uint128> {
    let validator_set = VALIDATOR_SET.load(storage)?;
    Ok(Uint128::from(validator_set.query_rewards(querier, contract.to_string())?))
}

pub fn stake_msg(validator: &str, amount: u128) -> CosmosMsg {
    CosmosMsg::Staking(StakingMsg::Delegate {
        validator: validator.to_string(),
        amount: Coin {
            denom: "ujuno".to_string(),
            amount: Uint128::from(amount),
        },
    })
}

pub fn undelegate_msg(validator: &str, amount: u128) -> CosmosMsg {
    CosmosMsg::Staking(StakingMsg::Undelegate {
        validator: validator.to_string(),
        amount: Coin {
            denom: "ujuno".to_string(),
            amount: Uint128::from(amount),
        },
    })
}

pub fn withdraw_msg(validator: &str) -> CosmosMsg {
    CosmosMsg::Distribution(DistributionMsg::WithdrawDelegatorReward {
        validator: validator.to_string(),
    })
}

pub fn redelegate_msg(from: &str, to: &str, amount: u128) -> CosmosMsg {
    CosmosMsg::Staking(StakingMsg::Redelegate {
        src_validator: from.to_string(),
        amount: Coin {
            denom: "ujuno".to_string(),
            amount: Uint128::from(amount),
        },
        dst_validator: to.to_string(),
    })
}
