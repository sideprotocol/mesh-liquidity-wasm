use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Uint128
};

use crate::error::ContractError;
use crate::msg::{ CallbackMsg, ExecuteMsg, InstantiateMsg, QueryMsg, SwapRequest};
use crate::state::{Constants, CONSTANTS};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
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
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::MultiSwap { requests, offer_amount, receiver, minimum_receive }
        => multi_swap(deps, env, info, requests, offer_amount, receiver, minimum_receive),
        ExecuteMsg::Reset { count } => try_reset(deps, env, info, count),
        ExecuteMsg::Callback(msg) => handle_callback(deps, env, info, msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
           QueryMsg::GetCount {} => query_count(deps),
    }
}

fn handle_callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: CallbackMsg,
) -> Result<Response, ContractError> {
    // Callback functions can only be called by contract only
    if info.sender != env.contract.address {
        return Err(ContractError::Std(StdError::generic_err(
            "callbacks cannot be invoked externally",
        )));
    }
    
    match msg {
        CallbackMsg::HopSwap {
            requests,
            offer_asset,
            prev_ask_amount,
            recipient,
            minimum_receive,
        } => hop_swap(
            deps,
            env,
            info,
            requests,
            offer_asset,
            prev_ask_amount,
            recipient,
            minimum_receive,
        ),
    }

}

fn hop_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    requests: Vec<SwapRequest>,
    offer_asset: String,
    prev_ask_amount: Uint128,
    recipient: Addr,
    minimum_receive: Uint128,
) -> Result<Response, ContractError> {



    Ok(Response::new()
    .add_attribute("action", "hop_swap"))
}

fn multi_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    requests: Vec<SwapRequest>,
    offer_amount: Uint128,
    receiver: Option<Addr>,
    minimum_receive: Option<Uint128>,
) -> Result<Response, ContractError> {
    let mut constant = CONSTANTS.load(deps.storage)?;
    constant.count += 1;
    CONSTANTS.save(deps.storage,&constant)?;
    Ok(Response::new()
        .add_attribute("action", "multi_swap"))
}

fn try_reset(
    deps: DepsMut,
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

// pub fn query_count(deps: Deps) -> StdResult<Binary> {
//     let res = deps.querier.query_params()?;
//     to_binary(&(res))
// }

// pub fn query_count(deps: Deps<SideQueryWrapper>) -> StdResult<Binary> {
//     let querier: SideQuerier<'_> = SideQuerier::new(&deps.querier);
//     let res = querier.query_params()?;
//     to_binary(&(res))
// }
