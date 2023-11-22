use cosmwasm_std::{DepsMut, Env, MessageInfo, Addr, Response, StdError, Uint128};

use crate::ContractError;
use crate::query::{fetch_validator_set_from_contract, query_delegation};
use crate::types::config::CONFIG;
use crate::types::validator_set::{VALIDATOR_SET};

/**
 * Update seJUNO token addr in config
 */
pub fn try_update_sejuno_addr(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: Addr,
) -> Result<Response, ContractError> {
    
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Std(StdError::generic_err(
            "Only admin can update sejuno Address"
        )));
    }

    config.sejuno_token = Some(address);
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
}

/**
 * Update bJUNO token addr in config
 */
pub fn try_update_bjuno_addr(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: Addr,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Std(StdError::generic_err(
            "Only admin can update bjuno Address"
        )));
    }

    config.bjuno_token = Some(address);
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
}

/**
 * Update top validator query contract addr in config
 */
pub fn try_update_validator_set_addr(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: Addr,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Std(StdError::generic_err(
            "Only admin can update Validator set"
        )));
    }

    if config.top_validator_contract.is_none() {  // only update the val set if was non before
        config.top_validator_contract = Some(address);
        CONFIG.save(deps.storage, &config)?;

        let validator_list = fetch_validator_set_from_contract(deps.as_ref())?;
        //UPDATE NEW LIST DATA
        VALIDATOR_SET.save(deps.storage, &validator_list)?;
    } else {
        config.top_validator_contract = Some(address);
        CONFIG.save(deps.storage, &config)?;
    }

    Ok(Response::new())
}

/**
 * Update rewards contract addr in config
 */
pub fn try_update_rewards_addr(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: Addr,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Std(StdError::generic_err(
            "Only admin can update reward Address"
        )));
    }

    config.rewards_contract = Some(address);
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
}

/**
* Rebalance staked amount according to current onchain delegation 
*/
pub fn rebalance_slash(
    deps: DepsMut,
    env: Env,
) -> Result<Response, ContractError> {
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