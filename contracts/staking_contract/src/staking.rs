use std::convert::TryFrom;
use std::u128;

use cosmwasm_std::{Addr, Coin, CosmosMsg, StakingMsg, StdError, StdResult, Storage, Uint128};
use cosmwasm_std::{CustomQuery, DistributionMsg, QuerierWrapper};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;

use crate::deposit::calc_fee;
use crate::state::{LSSIDE_FROZEN_TOKENS, LSSIDE_FROZEN_TOTAL_ONCHAIN, STATE};
use crate::tokens::query_total_supply;
use crate::types::config::CONFIG;
use crate::types::killswitch::KillSwitch;
use crate::types::validator_set::VALIDATOR_SET;

pub fn lsside_exchange_rate<Q: CustomQuery>(
    store: &dyn Storage,
    querier: QuerierWrapper<Q>,
) -> StdResult<Decimal> {
    let config = CONFIG.load(store)?;
    let state = STATE.load(store)?;
    let contract_address = config.contract_addr;

    let lsside_token = config
        .ls_side_token
        .ok_or_else(|| StdError::generic_err("lsSIDE token addr not registered".to_string()))?;

    if KillSwitch::try_from(config.kill_switch)? == KillSwitch::Closed {
        let total_on_chain = get_onchain_balance_with_rewards(querier, store, &contract_address)?;
        let tokens = query_total_supply(querier, &lsside_token)?
            .u128()
            .saturating_sub(state.lsside_to_burn.u128());
        let exchange_rate = _calc_exchange_rate(total_on_chain, tokens)?;
        Ok(exchange_rate)
    } else {
        let total_on_chain = LSSIDE_FROZEN_TOTAL_ONCHAIN.load(store)?.u128();
        let tokens = LSSIDE_FROZEN_TOKENS.load(store)?.u128();

        let exchange_rate = _calc_exchange_rate(total_on_chain, tokens)?;
        Ok(exchange_rate)
    }
}

pub fn _calc_exchange_rate(total_on_chain: u128, tokens: u128) -> StdResult<Decimal> {
    let side_balance = Decimal::from(total_on_chain as u64);
    let token_bal = Decimal::from(tokens as u64);

    let ratio = if total_on_chain == 0 || tokens == 0 {
        Decimal::one()
    } else {
        side_balance.checked_div(token_bal).unwrap()
    };

    Ok(ratio.round_dp_with_strategy(12, RoundingStrategy::AwayFromZero))
}

// return net_onchain balance with rewards lsside using backing to calculate amount
// check if we can also update backing here if total_staked differ with backing data
pub fn get_onchain_balance_with_rewards<Q: CustomQuery>(
    querier: QuerierWrapper<Q>,
    storage: &dyn Storage,
    contract_address: &Addr,
) -> StdResult<u128> {
    let config = CONFIG.load(storage)?;
    let state = STATE.load(storage)?;

    let rewards_balance = get_rewards(storage, querier, contract_address).unwrap_or_default();

    let fee = calc_fee(rewards_balance, config.dev_fee);
    let final_rewards = Uint128::from(rewards_balance.u128().saturating_sub(fee as u128));

    let lsside_onchain_balance = state.lsside_backing + Uint128::from(final_rewards.u128());
    Ok(lsside_onchain_balance.u128())
}

pub fn get_total_onchain_balance(storage: &dyn Storage) -> StdResult<u128> {
    let validator_set = VALIDATOR_SET.load(storage)?;
    let locked_balance = validator_set.total_staked();

    let state = STATE.load(storage)?;
    let to_deposit_balance = state.to_deposit.u128();

    Ok(locked_balance + to_deposit_balance)
}

pub fn get_balance<Q: CustomQuery>(
    querier: QuerierWrapper<Q>,
    address: &Addr,
) -> StdResult<Uint128> {
    let balance = querier.query_balance(address.clone(), &"uside".to_string())?;

    Ok(balance.amount)
}

pub fn get_rewards<Q: CustomQuery>(
    storage: &dyn Storage,
    querier: QuerierWrapper<Q>,
    contract: &Addr,
) -> StdResult<Uint128> {
    let validator_set = VALIDATOR_SET.load(storage)?;
    Ok(Uint128::from(
        validator_set.query_rewards(querier, contract.to_string())?,
    ))
}

pub fn stake_msg(validator: &str, amount: u128) -> CosmosMsg {
    CosmosMsg::Staking(StakingMsg::Delegate {
        validator: validator.to_string(),
        amount: Coin {
            denom: "uside".to_string(),
            amount: Uint128::from(amount),
        },
    })
}

pub fn undelegate_msg(validator: &str, amount: u128) -> CosmosMsg {
    CosmosMsg::Staking(StakingMsg::Undelegate {
        validator: validator.to_string(),
        amount: Coin {
            denom: "uside".to_string(),
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
            denom: "uside".to_string(),
            amount: Uint128::from(amount),
        },
        dst_validator: to.to_string(),
    })
}
