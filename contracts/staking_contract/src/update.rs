use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Response, StdError, Uint128};

use crate::query::query_delegation;
use crate::types::config::CONFIG;
use crate::types::validator_set::VALIDATOR_SET;
use crate::ContractError;

/**
 * Update lsside token addr in config
 */
pub fn try_update_lsside_addr(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: Addr,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Std(StdError::generic_err(
            "Only admin can update lsside Address",
        )));
    }

    config.ls_side_token = Some(address);
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
}

/**
* Rebalance staked amount according to current onchain delegation
*/
pub fn rebalance_slash(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let mut validator_set = VALIDATOR_SET.load(deps.storage)?;
    for val in validator_set.validators.iter_mut() {
        let current_amount = query_delegation(&deps.querier, &val.address, &env.contract.address)?;
        // what if zero ?
        if current_amount > 0 {
            val.staked = Uint128::from(current_amount);
        }
    }

    validator_set.rebalance();
    VALIDATOR_SET.save(deps.storage, &validator_set)?;
    Ok(Response::new())
}
