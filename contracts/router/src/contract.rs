use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, StdError, Querier
};

use crate::error::ContractError;
use crate::msg::{ ExecuteMsg, InstantiateMsg, QueryMsg, CountResponse};
use crate::state::{Constants, CONSTANTS};
use crate::querier::SideQuerier;
use crate::query::SideQueryWrapper;

#[entry_point]
pub fn instantiate(
    deps: DepsMut<SideQueryWrapper>,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = Constants {
        count: msg.count,
        owner: _info.sender.to_string()
    };
    CONSTANTS.save(deps.storage,&state)?;
    Ok(Response::new()
        .add_attribute("action", "initialisation")
        .add_attribute("sender", _info.sender.clone()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<SideQueryWrapper>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Increment {} => try_increment(deps, env, info),
        ExecuteMsg::Reset { count } => try_reset(deps, env, info, count),
        ExecuteMsg::Callback {} => handle_callback(deps, env, info),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<SideQueryWrapper>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
           QueryMsg::GetCount {} => query_count(deps),
    }
}

fn handle_callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // Callback functions can only be called by contract only
    if info.sender != env.contract.address {
        return Err(ContractError::Std(StdError::generic_err(
            "callbacks cannot be invoked externally",
        )));
    }
    
}

fn try_increment(
    deps: DepsMut<SideQueryWrapper>,
    _env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    let mut constant = CONSTANTS.load(deps.storage)?;
    constant.count += 1;
    CONSTANTS.save(deps.storage,&constant)?;
    Ok(Response::new()
        .add_attribute("action", "increment"))
}

fn try_reset(
    deps: DepsMut<SideQueryWrapper>,
    _env: Env,
    info: MessageInfo,
    count: i32
) -> Result<Response, ContractError> {
    let mut constant = CONSTANTS.load(deps.storage)?;
    if constant.owner != info.sender {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized",
        )));
    }
    constant.count = count;
    CONSTANTS.save(deps.storage, & constant)?;
    Ok(Response::new()
        .add_attribute("action", "COUNT reset successfully"))
}

pub fn query_count(deps: Deps<SideQueryWrapper>) -> StdResult<Binary> {
    let querier: SideQuerier<'_> = SideQuerier::new(&deps.querier);
    let res = querier.query_params()?;
    to_binary(&(res))
}

// pub fn query_count(deps: Deps<SideQueryWrapper>) -> StdResult<Binary> {
//     let querier: SideQuerier<'_> = SideQuerier::new(&deps.querier);
//     let res = querier.query_params()?;
//     to_binary(&(res))
// }
