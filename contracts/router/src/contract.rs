use cosmwasm_std::{
    entry_point, to_binary, Addr, BalanceResponse, BankMsg, BankQuery, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, QueryRequest, Response, StdError, StdResult, Uint128, WasmMsg
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
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = Constants {
        owner: _info.sender.to_string()
    };
    CONSTANTS.save(deps.storage,&state)?;
    Ok(Response::new()
        .add_attribute("action", "initialisation")
        .add_attribute("sender", _info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<SideMsg>, ContractError> {
    match msg {
        ExecuteMsg::MultiSwap { requests, offer_amount, receiver, minimum_receive }
        => multi_swap(deps, env, info, requests, offer_amount, receiver, minimum_receive),
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
) -> Result<Response<SideMsg>, ContractError> {
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
    _info: MessageInfo,
    mut requests: Vec<SwapRequest>,
    offer_asset: String,
    prev_ask_amount: Uint128,
    recipient: Addr,
    minimum_receive: Uint128,
) -> Result<Response<SideMsg>, ContractError> {

    // Calculate current offer asset balance
    let asset_balance = query_balance(&deps.querier, env.contract.address.clone(), offer_asset.clone())?;

    // Amount returned from the last hop swap
    let amount_returned_prev_hop = asset_balance.checked_sub(prev_ask_amount).unwrap();

    // ExecuteMsgs
    let mut execute_msgs: Vec<CosmosMsg<SideMsg>> = vec![];
    let current_ask_balance: Uint128;

    // If Hop is over, check if the minimum receive amount is met and transfer the tokens to the recipient
    if requests.is_empty() {
        if amount_returned_prev_hop < minimum_receive {
            return Err(ContractError::InvalidMultihopSwapRequest {
                msg: format!("Minimum receive amount not met. Swap failed. Amount received = {} Minimum receive amount = {}", amount_returned_prev_hop, minimum_receive),
            });
        }
        execute_msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: recipient.to_string(),
            amount: vec![Coin {
                denom: offer_asset,
                amount: amount_returned_prev_hop,
            }],
        }));
    } else {
        let next_hop = requests[0].clone();

        // Asset returned from prev hop needs to match the asset to be used for the next hop
        if offer_asset != next_hop.asset_in {
            return Err(ContractError::InvalidMultihopSwapRequest {
                msg:
                format!("Invalid multiswap request. Asset {} out of previous hop does not match the asset {} to be provided for next hop."
                , offer_asset, next_hop.asset_in),
            });
        }

        let token_in: Coin = Coin { denom: next_hop.asset_in, amount: amount_returned_prev_hop };
        let token_out: Coin = Coin { denom: next_hop.asset_out.clone(), amount: Uint128::from(1u64) };
    
        let swap_msg = CosmosMsg::Custom(SideMsg::Swap {
            pool_id: next_hop.pool_id,
            token_in, token_out, slippage: "99".to_string() 
        });
        execute_msgs.push(swap_msg);

        // Get current balance of the ask asset (Native) token
        current_ask_balance = query_balance(&deps.querier, env.contract.address.clone(), requests[0].asset_out.clone())?;

        // Add Callback Msg as we need to continue with the hops
        requests.remove(0);
        let arb_chain_msg = CallbackMsg::HopSwap {
            requests,
            offer_asset: next_hop.asset_out,
            prev_ask_amount: current_ask_balance,
            recipient,
            minimum_receive,
        };
        let arb_chain = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: String::from(env.contract.address),
            msg: to_binary(&ExecuteMsg::Callback(arb_chain_msg))?,
            funds: vec![],
        });
        execute_msgs.push(arb_chain);
    }

    Ok(Response::new()
        .add_attribute("action", "hop_swap")
        .add_messages(execute_msgs)
    )
}

fn multi_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mut requests: Vec<SwapRequest>,
    offer_amount: Uint128,
    receiver: Option<Addr>,
    minimum_receive: Option<Uint128>,
) -> Result<Response<SideMsg>, ContractError> {
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
    let mut execute_msgs: Vec<CosmosMsg<SideMsg>> = vec![];
    let minimum_receive = minimum_receive.unwrap_or(Uint128::zero());

    // Current ask token balance available with the router contract
    

    // Check sent tokens
    // Query - Get number of offer asset (Native) tokens sent with the msg
    // check if given tokens are received here
    let mut tokens_received = Uint128::from(0u64);
    let mut ok = false;
    for asset in info.funds {
        if asset.denom == requests[0].asset_in {
            ok = true;
            tokens_received = asset.amount;
        }
    }
    if !ok {
        return Err(ContractError::Std(StdError::generic_err(
            "Funds not found: Funds not sent"
                .to_string(),
        )));
    }

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

    let token_in: Coin = Coin { denom: first_hop.asset_in, amount: tokens_received };
    let token_out: Coin = Coin { denom: first_hop.asset_out.clone(), amount: Uint128::from(1u64) };

    let swap_msg = CosmosMsg::Custom(SideMsg::Swap {
        pool_id: first_hop.pool_id,
        token_in, token_out, slippage: "99".to_string()  
    });

    execute_msgs.push(swap_msg);

    // Get current balance of the ask asset (Native) token
    let current_ask_balance: Uint128 = query_balance(&deps.querier, env.contract.address.clone(), requests[0].asset_out.clone())?;

    // CallbackMsg - Add Callback Msg as we need to continue with the hops
    requests.remove(0);
    let arb_chain_msg = CallbackMsg::HopSwap {
        requests,
        offer_asset: first_hop.asset_out,
        prev_ask_amount: current_ask_balance,
        recipient,
        minimum_receive,
    };

    let arb_chain = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: String::from(env.contract.address),
        msg: to_binary(&ExecuteMsg::Callback(arb_chain_msg))?,
        funds: vec![],
    });
    execute_msgs.push(arb_chain);

    Ok(Response::new()
        .add_messages(execute_msgs)
        .add_attribute("action", "multi_swap")
        .add_attribute("offer_amount", offer_amount.to_string())
        .add_attribute("minimum_receive", minimum_receive.to_string())
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

/// ## Description
/// Returns the balance of the denom at the specified account address.
/// ## Params
/// * **querier** is the object of type [`QuerierWrapper`].
/// * **account_addr** is the object of type [`Addr`].
/// * **denom** is the object of type [`String`].
pub fn query_balance(
    querier: &QuerierWrapper,
    account_addr: Addr,
    denom: String,
) -> StdResult<Uint128> {
    let balance: BalanceResponse = querier.query(&QueryRequest::Bank(BankQuery::Balance {
        address: String::from(account_addr),
        denom,
    }))?;
    Ok(balance.amount.amount)
}
