use cosmwasm_std::{
    entry_point, to_binary, Addr, Api, Binary, CosmosMsg, Deps, DepsMut, Env, Event, MessageInfo, Response, StdError, StdResult, Uint128
};

use crate::error::ContractError;
use crate::interaction_gmm::SideMsg;
use crate::msg::{ CallbackMsg, ExecuteMsg, InstantiateMsg, QueryMsg, SwapRequest};
use crate::querier::SideQuerier;
use crate::query::SideQuery;
use crate::state::{Constants, CONSTANTS};

pub const MAX_SWAP_OPERATIONS: usize = 50;

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
pub fn query(deps: Deps<SideQuery>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
           QueryMsg::GetCount {} => query_param(deps),
    }
}

pub fn query_param(deps: Deps<SideQuery>) -> StdResult<Binary> {
    let querier: SideQuerier<'_> = SideQuerier::new(&deps.querier);
    let res = querier.query_params()?;
    to_binary(&(res))
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
    let recipient = deps.api.addr_validate(receiver.unwrap_or(info.sender.clone()).as_str())?;

    // Validate the multiswap request
    let requests_len = requests.len();
    if requests_len < 1 {
        return Err(ContractError::InvalidMultihopSwapRequest {
            msg: "Multihop swap request must contain at least 1 hop".to_string(),
        });
    }

    if requests_len > MAX_SWAP_OPERATIONS {
        return Err(ContractError::InvalidMultihopSwapRequest {
            msg: "The swap operation limit was exceeded!".to_string(),
        });
    }

    // Assert the requests are properly set
    assert_requests(&requests)?;

    // CosmosMsgs to be sent in the response
    let mut execute_msgs: Vec<CosmosMsg> = vec![];
    let minimum_receive = minimum_receive.unwrap_or(Uint128::zero());

    // Current ask token balance available with the router contract
    let current_ask_balance: Uint128;

    // Check sent tokens
    // Query - Get number of offer asset (Native) tokens sent with the msg
    let tokens_received = requests[0]
    .asset_in
    .get_sent_native_token_balance(&info);

    // Error - if the number of native tokens sent is less than the offer amount, then return error
    if tokens_received < offer_amount {
        return Err(ContractError::InvalidMultihopSwapRequest {
            msg: format!(
                "Invalid number of tokens sent. The offer amount is larger than the number of tokens received. Tokens received = {} Tokens offered = {}",
                tokens_received, offer_amount
            ),
        });
    }

    
    // Create SingleSwapRequest for the first hop
    let first_hop = requests[0].clone();
    // let first_hop_swap_request = SingleSwapRequest {
    //     pool_id: first_hop.pool_id,
    //     asset_in: first_hop.asset_in.clone(),
    //     asset_out: first_hop.asset_out.clone(),
    //     swap_type: SwapType::GiveIn {},
    //     // Amount provided is the amount to be used for the first hop
    //     amount: offer_amount,
    //     max_spread: first_hop.max_spread,
    //     belief_price: first_hop.belief_price,
    // };

    // Need to send native tokens if the offer asset is native token
    // ExecuteMsg - For the first hop
    // let first_hop_execute_msg = CosmosMsg::Wasm(WasmMsg::Execute {
    //     contract_addr: config.dexter_vault.to_string(),
    //     funds: coins,
    //     msg: to_json_binary(&vault::ExecuteMsg::Swap {
    //         swap_request: first_hop_swap_request.clone(),
    //         recipient: Some(env.contract.address.clone().to_string()),
    //         min_receive: None,
    //         max_spend: None,
    //     })?,
    // });
    // execute_msgs.push(first_hop_execute_msg);

    SideMsg::Swap {
        pool_id: first_hop.pool_id,
        token_in: first_hop.asset_in, token_out: first_hop.asset_out };

    // Get current balance of the ask asset (Native) token
    current_ask_balance = requests[0]
        .asset_out
        .query_for_balance(&deps.querier, env.contract.address.clone())?;

    // CallbackMsg - Add Callback Msg as we need to continue with the hops
    requests.remove(0);
    let arb_chain_msg = CallbackMsg::HopSwap {
        requests: requests,
        offer_asset: first_hop.asset_out,
        prev_ask_amount: current_ask_balance,
        recipient,
        minimum_receive,
    }
    .to_cosmos_msg(&env.contract.address)?;
    execute_msgs.push(arb_chain_msg);

    let mut constant = CONSTANTS.load(deps.storage)?;
    constant.count += 1;
    CONSTANTS.save(deps.storage,&constant)?;
    Ok(Response::new()
        .add_attribute("action", "multi_swap")
        .add_attribute("offer_amount", offer_amount.to_string())
        .add_attribute("recipient", recipient.to_string())
        .add_attribute("minimum_receive", minimum_receive.to_string())
        .add_attribute("hops_left", requests.len().to_string())
    )
}

/// Validates swap requests.
///
/// * **requests** is a vector that contains objects of type [`HopSwapRequest`]. These are all the swap operations we check.
fn assert_requests(requests: &[SwapRequest]) -> Result<(), ContractError> {
    let mut prev_req: SwapRequest = requests[0].clone();

    for i in 1..requests.len() {
        if requests[i].asset_in != prev_req.asset_out {
            return Err(ContractError::InvalidMultihopSwapRequest {
                msg: "invalid sequence of requests; prev output doesn't match current input".to_string()
            });
        }
        prev_req = requests[i].clone();
    }

    Ok(())
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
