#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, IbcMsg, IbcTimeout, MessageInfo, Order, Response,
    StdResult, StdError,
};

use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{
    AtomicSwapPacketData, CancelSwapMsg, DetailsResponse, ExecuteMsg, InstantiateMsg, ListResponse,
    MakeSwapMsg, QueryMsg, SwapMessageType, TakeSwapMsg, MigrateMsg, MakeBidMsg, TakeBidMsg, CancelBidMsg,
};
use crate::state::{
    AtomicSwapOrder,
    Status,
    CHANNEL_INFO,
    append_atomic_order,
    SWAP_ORDERS, set_atomic_order, get_atomic_order, COUNT, move_order_to_bottom, BID_ORDER_TO_COUNT, Bid, BIDS, INACTIVE_SWAP_ORDERS, Side,
};
use crate::utils::{generate_order_id,extract_source_channel_for_taker_msg,order_path};
use cw_storage_plus::Bound;

// Version info, for migration info
const CONTRACT_NAME: &str = "ics100-swap";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_TIMEOUT_TIMESTAMP_OFFSET: u64 = 600;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    COUNT.save(deps.storage, &0u64)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::MakeSwap(msg) => execute_make_swap(deps, env, info, msg),
        ExecuteMsg::TakeSwap(msg) => execute_take_swap(deps, env, info, msg),
        ExecuteMsg::CancelSwap(msg) => execute_cancel_swap(deps, env, info, msg),
        ExecuteMsg::MakeBid(msg) => execute_make_bid(deps, env, info, msg),
        ExecuteMsg::TakeBid(msg) => execute_take_bid(deps, env, info, msg),
        ExecuteMsg::CancelBid(msg) => execute_cancel_bid(deps, env, info, msg),
    }
}

// MakeSwap is called when the maker wants to make atomic swap. The method create new order and lock tokens.
// This is the step 1 (Create order & Lock Token) of the atomic swap: https://github.com/cosmos/ibc/tree/main/spec/app/ics-100-atomic-swap
pub fn execute_make_swap(
    _deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: MakeSwapMsg,
) -> Result<Response, ContractError> {
    // check if given tokens are received here
    let mut ok = false;
    // First token in this chain only first token needs to be verified
    for asset in info.funds {
        if asset.denom == msg.sell_token.denom && msg.sell_token.amount == asset.amount {
            ok = true;
        }
    }
    if !ok {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Funds mismatch: Funds mismatched to with message and sent values: Make swap"
        ))));
    }

    // TODO: I guess we have add order here with status - Initial. 
    // This optimistic operation will update user's experiences in frontend code. 
    // When success update status as a Sync. when failed, we can remove that pool with refund logic. 
    
    let channel_info = CHANNEL_INFO.load(_deps.storage, &msg.source_channel)?;
    let path = order_path(
        msg.source_channel.clone(),
        msg.source_port.clone(),
        channel_info.counterparty_endpoint.channel_id,
        channel_info.counterparty_endpoint.port_id,
    )?;

    let order_id = generate_order_id(&path)?;
    let new_order = AtomicSwapOrder {
        id: order_id.clone(),
        side: Side::Native,
        maker: msg.clone(),
        status: Status::Initial,
        path: path.clone(),
        taker: None,
        cancel_timestamp: None,
        complete_timestamp: None,
        create_timestamp: env.block.time.seconds()
    };

    append_atomic_order(_deps.storage, &order_id, &new_order)?;

    let ibc_packet = AtomicSwapPacketData {
        r#type: SwapMessageType::MakeSwap,
        data: to_binary(&msg)?,
        order_id: Some(order_id),
        path: Some(path),
        memo: String::new(),
    };

    let ibc_msg = IbcMsg::SendPacket {
        channel_id: msg.source_channel.clone(),
        data: to_binary(&ibc_packet)?,
        timeout: IbcTimeout::from(
            env.block
                .time
                .plus_seconds(DEFAULT_TIMEOUT_TIMESTAMP_OFFSET),
        ),
    };

    let res = Response::new()
        .add_message(ibc_msg)
        .add_attribute("action", "make_swap");
    Ok(res)
}

// TakeSwap is the step 5 (Lock Order & Lock Token) of the atomic swap: https://github.com/liangping/ibc/blob/atomic-swap/spec/app/ics-100-atomic-swap/ibcswap.png
// This method lock the order (set a value to the field "Taker") and lock Token
pub fn execute_take_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: TakeSwapMsg,
) -> Result<Response, ContractError> {
    // check if given tokens are received here
    let mut ok = false;
    // First token in this chain only first token needs to be verified
    for asset in info.funds {
        if asset.denom == msg.sell_token.denom && msg.sell_token.amount == asset.amount {
            ok = true;
        }
    }
    if !ok {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Funds mismatch: Funds mismatched to with message and sent values: Make swap"
        ))));
    }

    let mut order = get_atomic_order(deps.storage, &msg.order_id)?;

    if order.status != Status::Initial && order.status != Status::Sync {
        return Err(ContractError::OrderTaken);
    }

    // Make sure the maker's buy token matches the taker's sell token
    if order.maker.buy_token != msg.sell_token {
        return Err(ContractError::InvalidSellToken);
    }

    // Checks if the order has already been taken
    if let Some(_taker) = order.taker {
        return Err(ContractError::OrderTaken);
    }

    // If `desiredTaker` is set, only the desiredTaker can accept the order.
    if order.maker.desired_taker != "" && order.maker.desired_taker != msg.clone().taker_address {
        return Err(ContractError::InvalidTakerAddress);
    }

    if env.block.time.seconds() > order.maker.expiration_timestamp {
        move_order_to_bottom(deps.storage, &msg.order_id)?;
        return Err(ContractError::Expired);
    }

    order.taker = Some(msg.clone());

    let ibc_packet = AtomicSwapPacketData {
        r#type: SwapMessageType::TakeSwap,
        data: to_binary(&msg)?,
        order_id: None,
        path: None,
        memo: String::new(),
    };

    let ibc_msg = IbcMsg::SendPacket {
        channel_id: extract_source_channel_for_taker_msg(&order.path)?,
        data: to_binary(&ibc_packet)?,
        timeout: IbcTimeout::from(
            env.block
                .time
                .plus_seconds(DEFAULT_TIMEOUT_TIMESTAMP_OFFSET),
        ),
    };

    // Save order
    set_atomic_order(deps.storage, &order.id, &order)?;

    let res = Response::new()
        .add_message(ibc_msg)
        .add_attribute("order_id", msg.order_id)
        .add_attribute("action", "take_swap")
        .add_attribute("order_id", order.id.clone());
    return Ok(res);
}

// CancelSwap is the step 10 (Cancel Request) of the atomic swap: https://github.com/cosmos/ibc/tree/main/spec/app/ics-100-atomic-swap.
// It is executed on the Maker chain. Only the maker of the order can cancel the order.
pub fn execute_cancel_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: CancelSwapMsg,
) -> Result<Response, ContractError> {
    let sender = info.sender.to_string();
    let order = get_atomic_order(deps.storage, &msg.order_id)?;

    if sender != order.maker.maker_address {
        return Err(ContractError::InvalidSender);
    }

    // Make sure the sender is the maker of the order.
    if order.maker.maker_address != msg.maker_address {
        return Err(ContractError::InvalidMakerAddress);
    }

    // Make sure the order is in a valid state for cancellation
    if order.status != Status::Sync && order.status != Status::Initial {
        return Err(ContractError::InvalidStatus);
    }

    let packet = AtomicSwapPacketData {
        r#type: SwapMessageType::CancelSwap,
        data: to_binary(&msg)?,
        memo: String::new(),
        order_id: None,
        path: None
    };

    let ibc_msg = IbcMsg::SendPacket {
        channel_id: order.maker.source_channel,
        data: to_binary(&packet)?,
        timeout: IbcTimeout::from(
            env.block
                .time
                .plus_seconds(DEFAULT_TIMEOUT_TIMESTAMP_OFFSET),
        ),
    };

    let res = Response::new()
        .add_message(ibc_msg)
        .add_attribute("order_id", msg.order_id)
        .add_attribute("action", "cancel_swap")
        .add_attribute("order_id", order.id.clone());
    return Ok(res);
}

/// Make bid: Use it to make bid from taker chain
/// For each order each user can create atmost 1 bid(they can cancel and recreate it)
/// Panics id bid is already taken
/// buy_token != sell_token
pub fn execute_make_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: MakeBidMsg,
) -> Result<Response, ContractError> {
    let sender = info.sender.to_string();
    let order = get_atomic_order(deps.storage, &msg.order_id)?;
    // check if given tokens are received here
    let mut ok = false;
    // First token in this chain only first token needs to be verified
    for asset in info.funds {
        if asset.denom == msg.sell_token.denom && msg.sell_token.amount == asset.amount {
            ok = true;
        }
    }
    if !ok {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Funds mismatch: Funds mismatched to with message and sent values: Make swap"
        ))));
    }

    if !order.maker.take_bids {
        return Err(ContractError::TakeBidNotAllowed);
    }

    // Make sure the maker's buy token matches the taker's sell token
    if order.maker.buy_token.denom != msg.sell_token.denom {
        return Err(ContractError::InvalidSellToken);
    }

    // Checks if the order has already been taken
    if let Some(_taker) = order.taker {
        return Err(ContractError::OrderTaken);
    }

    if sender != msg.taker_address {
        return Err(ContractError::InvalidSender);
    }

    let key = msg.order_id.clone() + &msg.taker_address;
    if BID_ORDER_TO_COUNT.has(deps.storage, &key) {
        return Err(ContractError::BidAlreadyExist {});
    }

    let packet = AtomicSwapPacketData {
        r#type: SwapMessageType::MakeBid,
        data: to_binary(&msg)?,
        memo: String::new(),
        order_id: None,
        path: None
    };

    let ibc_msg = IbcMsg::SendPacket {
        channel_id: order.maker.source_channel,
        data: to_binary(&packet)?,
        timeout: IbcTimeout::from(
            env.block
                .time
                .plus_seconds(DEFAULT_TIMEOUT_TIMESTAMP_OFFSET),
        ),
    };

    let res = Response::new()
        .add_message(ibc_msg)
        .add_attribute("order_id", msg.order_id)
        .add_attribute("action", "make_bid");
    return Ok(res);
}

/// Take Bid: Only maker(maker receiving address) can take bid for their order
/// Maker can take bid from taker chain
/// Panics if order is already taken
/// Panics if is not allowed
/// Panics if bid doesn't exist or sender is not maker receiving address
pub fn execute_take_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: TakeBidMsg,
) -> Result<Response, ContractError> {
    let sender = info.sender.to_string();
    let order = get_atomic_order(deps.storage, &msg.order_id)?;

    if !order.maker.take_bids {
        return Err(ContractError::TakeBidNotAllowed);
    }

    // Checks if the order has already been taken
    if let Some(_taker) = order.taker {
        return Err(ContractError::OrderTaken);
    }

    if sender != order.maker.maker_receiving_address {
        return Err(ContractError::InvalidSender);
    }

    let key = msg.order_id.clone() + &msg.bidder;
    if !BID_ORDER_TO_COUNT.has(deps.storage, &key) {
        return Err(ContractError::BidDoesntExist);
    }

    let packet = AtomicSwapPacketData {
        r#type: SwapMessageType::TakeBid,
        data: to_binary(&msg)?,
        memo: String::new(),
        order_id: None,
        path: None
    };

    let ibc_msg = IbcMsg::SendPacket {
        channel_id: order.maker.source_channel,
        data: to_binary(&packet)?,
        timeout: IbcTimeout::from(
            env.block
                .time
                .plus_seconds(DEFAULT_TIMEOUT_TIMESTAMP_OFFSET),
        ),
    };

    let res = Response::new()
        .add_message(ibc_msg)
        .add_attribute("order_id", msg.order_id)
        .add_attribute("action", "take_bid");
    return Ok(res);
}

/// Cancel Bid: Bid maker can cancel their bid
/// After cancellation amount is refunded
/// Panics if bid is not allowed
/// bid doesn't exist
pub fn execute_cancel_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: CancelBidMsg,
) -> Result<Response, ContractError> {
    let sender = info.sender.to_string();
    let order = get_atomic_order(deps.storage, &msg.order_id)?;

    if !order.maker.take_bids {
        return Err(ContractError::TakeBidNotAllowed);
    }

    let key = msg.order_id.clone() + &sender;
    if !BID_ORDER_TO_COUNT.has(deps.storage, &key) {
        return Err(ContractError::BidDoesntExist);
    }

    if sender != msg.bidder {
        return Err(ContractError::InvalidSender);
    }

    let packet = AtomicSwapPacketData {
        r#type: SwapMessageType::CancelBid,
        data: to_binary(&msg)?,
        memo: String::new(),
        order_id: None,
        path: None
    };

    let ibc_msg = IbcMsg::SendPacket {
        channel_id: order.maker.source_channel,
        data: to_binary(&packet)?,
        timeout: IbcTimeout::from(
            env.block
                .time
                .plus_seconds(DEFAULT_TIMEOUT_TIMESTAMP_OFFSET),
        ),
    };

    let res = Response::new()
        .add_message(ibc_msg)
        .add_attribute("order_id", msg.order_id)
        .add_attribute("action", "make_bid");
    return Ok(res);
}

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let ver = cw2::get_contract_version(deps.storage)?;
    // ensure we are migrating from an allowed contract
    if ver.contract != CONTRACT_NAME {
        return Err(StdError::generic_err("Can only upgrade from same type").into());
    }
    // note: better to do proper semver compare, but string compare *usually* works
    if ver.version >= CONTRACT_VERSION.to_string() {
        return Err(StdError::generic_err("Cannot upgrade from a newer version").into());
    }

    // set the new version
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::List { start_after, limit, order } => to_binary(&query_list(deps, start_after, limit, order)?),
        QueryMsg::ListByDesiredTaker {
            start_after,
            limit,
            desired_taker,
        } => to_binary(&query_list_by_desired_taker(
            deps,
            start_after,
            limit,
            desired_taker,
        )?),
        QueryMsg::ListByMaker {
            start_after,
            limit,
            maker,
        } => to_binary(&query_list_by_maker(deps, start_after, limit, maker)?),
        QueryMsg::ListByTaker {
            start_after,
            limit,
            taker,
        } => to_binary(&query_list_by_taker(deps, start_after, limit, taker)?),
        QueryMsg::Details { id } => to_binary(&query_details(deps, id)?),
        QueryMsg::BidDetailsbyOrder { start_after, limit, order_id }
            => to_binary(&query_bids_by_order(deps, start_after, limit, order_id)?),
        QueryMsg::BidDetailsbyBidder { order_id, bidder }
            => to_binary(&query_bids_by_bidder(deps,  order_id, bidder)?),
        // Inactive fields
        QueryMsg::InactiveList { start_after, limit, order } => to_binary(&query_inactive_list(deps, start_after, limit, order)?),
        QueryMsg::InactiveListByDesiredTaker {
            start_after,
            limit,
            desired_taker,
        } => to_binary(&query_inactive_list_by_desired_taker(
            deps,
            start_after,
            limit,
            desired_taker,
        )?),
        QueryMsg::InactiveListByMaker {
            start_after,
            limit,
            maker,
        } => to_binary(&query_inactive_list_by_maker(deps, start_after, limit, maker)?),
        QueryMsg::InactiveListByTaker {
            start_after,
            limit,
            taker,
        } => to_binary(&query_inactive_list_by_taker(deps, start_after, limit, taker)?),
    }
}

fn query_details(deps: Deps, id: String) -> StdResult<DetailsResponse> {
    let swap_order = get_atomic_order(deps.storage, &id)?;

    let details = DetailsResponse {
        id,
        maker: swap_order.maker.clone(),
        status: swap_order.status.clone(),
        path: swap_order.path.clone(),
        taker: swap_order.taker.clone(),
        cancel_timestamp: swap_order.cancel_timestamp.clone(),
        complete_timestamp: swap_order.complete_timestamp.clone(),
    };
    Ok(details)
}

// Settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

fn query_list(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
    order: Option<String>
) -> StdResult<ListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|s| Bound::ExclusiveRaw(s.into_bytes()));
    let order = order.unwrap_or("asc".to_string());
    let list_order;
    if order == "asc".to_string() {
        list_order = Order::Ascending;
    } else {
        list_order = Order::Descending;
    }
    let swap_orders = SWAP_ORDERS
        .range(deps.storage, start, None, list_order)
        .take(limit)
        .map(|item: Result<(u64, AtomicSwapOrder), cosmwasm_std::StdError>| item.unwrap().1)
        .collect::<Vec<AtomicSwapOrder>>();

    Ok(ListResponse { swaps: swap_orders })
}

fn query_list_by_desired_taker(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
    desired_taker: String,
) -> StdResult<ListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|s| Bound::ExclusiveRaw(s.into_bytes()));
    let swap_orders = SWAP_ORDERS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item: Result<(u64, AtomicSwapOrder), cosmwasm_std::StdError>| item.unwrap().1)
        .filter(|swap_order| swap_order.maker.desired_taker == desired_taker)
        .collect::<Vec<AtomicSwapOrder>>();

    Ok(ListResponse { swaps: swap_orders })
}

fn query_list_by_maker(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
    maker: String,
) -> StdResult<ListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|s| Bound::ExclusiveRaw(s.into_bytes()));
    let swap_orders = SWAP_ORDERS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item: Result<(u64, AtomicSwapOrder), cosmwasm_std::StdError>| item.unwrap().1)
        .filter(|swap_order| swap_order.maker.maker_address == maker)
        .collect::<Vec<AtomicSwapOrder>>();

    Ok(ListResponse { swaps: swap_orders })
}

fn query_list_by_taker(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
    taker: String,
) -> StdResult<ListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|s| Bound::ExclusiveRaw(s.into_bytes()));
    let swap_orders = SWAP_ORDERS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item: Result<(u64, AtomicSwapOrder), cosmwasm_std::StdError>| item.unwrap().1)
        .filter(|swap_order| {
            swap_order.taker.is_some() && swap_order.taker.clone().unwrap().taker_address == taker
        })
        .collect::<Vec<AtomicSwapOrder>>();

    Ok(ListResponse { swaps: swap_orders })
}

fn query_bids_by_order(
    deps: Deps,
    _start_after: Option<String>,
    limit: Option<u32>,
    order: String,
) -> StdResult<Vec<Bid>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let bids = BIDS.prefix(&order)
    .range(deps.storage, None, None, Order::Ascending)
    .take(limit)
    .map(|item| {
        item.map(|(_addr, bid)| {
            bid
        })
    })
    .collect::<StdResult<_>>()?;

    Ok(bids)
}

fn query_bids_by_bidder(
    deps: Deps,
    order: String,
    bidder: String,
) -> StdResult<Bid> {
    let key = order.clone() + &bidder;
    let count = BID_ORDER_TO_COUNT.load(deps.storage, &key)?;
    let bid = BIDS.load(deps.storage, (&order, &count.to_string()))?;

    Ok(bid)
}

// Inactive fields

fn query_inactive_list(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
    order: Option<String>
) -> StdResult<ListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|s| Bound::ExclusiveRaw(s.into_bytes()));
    let order = order.unwrap_or("asc".to_string());
    let list_order;
    if order == "asc".to_string() {
        list_order = Order::Ascending;
    } else {
        list_order = Order::Descending;
    }
    let swap_orders = INACTIVE_SWAP_ORDERS
        .range(deps.storage, start, None, list_order)
        .take(limit)
        .map(|item: Result<(u64, AtomicSwapOrder), cosmwasm_std::StdError>| item.unwrap().1)
        .collect::<Vec<AtomicSwapOrder>>();

    Ok(ListResponse { swaps: swap_orders })
}

fn query_inactive_list_by_desired_taker(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
    desired_taker: String,
) -> StdResult<ListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|s| Bound::ExclusiveRaw(s.into_bytes()));
    let swap_orders = INACTIVE_SWAP_ORDERS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item: Result<(u64, AtomicSwapOrder), cosmwasm_std::StdError>| item.unwrap().1)
        .filter(|swap_order| swap_order.maker.desired_taker == desired_taker)
        .collect::<Vec<AtomicSwapOrder>>();

    Ok(ListResponse { swaps: swap_orders })
}

fn query_inactive_list_by_maker(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
    maker: String,
) -> StdResult<ListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|s| Bound::ExclusiveRaw(s.into_bytes()));
    let swap_orders = INACTIVE_SWAP_ORDERS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item: Result<(u64, AtomicSwapOrder), cosmwasm_std::StdError>| item.unwrap().1)
        .filter(|swap_order| swap_order.maker.maker_address == maker)
        .collect::<Vec<AtomicSwapOrder>>();

    Ok(ListResponse { swaps: swap_orders })
}

fn query_inactive_list_by_taker(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
    taker: String,
) -> StdResult<ListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|s| Bound::ExclusiveRaw(s.into_bytes()));
    let swap_orders = INACTIVE_SWAP_ORDERS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item: Result<(u64, AtomicSwapOrder), cosmwasm_std::StdError>| item.unwrap().1)
        .filter(|swap_order| {
            swap_order.taker.is_some() && swap_order.taker.clone().unwrap().taker_address == taker
        })
        .collect::<Vec<AtomicSwapOrder>>();

    Ok(ListResponse { swaps: swap_orders })
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, from_binary, StdError};

    use crate::msg::{Height, TakeSwapMsgOutput};
    use crate::utils::{generate_order_id, order_path};

    use super::*;

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();

        // Instantiate an empty contract
        let instantiate_msg = InstantiateMsg {};
        let info = mock_info("anyone", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_make_swap() {
        let mut deps = mock_dependencies();

        let info = mock_info("anyone", &[]);
        let env = mock_env();
        instantiate(deps.as_mut(), env.clone(), info, InstantiateMsg {}).unwrap();

        let sender = String::from("sender0001");
        // let balance = coins(100, "tokens");
        let balance1 = coin(100, "token1");
        let balance2 = coin(200, "token2");
        let source_port = String::from("100");
        let source_channel = String::from("ics100-1");

        // Cannot create, no funds
        let info = mock_info(&sender, &[]);
        let create = MakeSwapMsg {
            source_port,
            source_channel,
            sell_token: balance1,
            buy_token: balance2,
            maker_address: "maker0001".to_string(),
            maker_receiving_address: "makerrcpt0001".to_string(),
            desired_taker: "".to_string(),
            expiration_timestamp: env.block.time.plus_seconds(100).nanos(),
            timeout_height: Height {
                revision_number: 0,
                revision_height: 0,
            },
            timeout_timestamp: env.block.time.plus_seconds(100).nanos(),
            take_bids: false
        };
        let err = execute(
            deps.as_mut(),
            env.clone(),
            info,
            ExecuteMsg::MakeSwap(create),
        )
        .unwrap_err();
        assert_eq!(err, ContractError::EmptyBalance {});
    }

    #[test]
    fn test_order_id() {
        let mut deps = mock_dependencies();

        let info = mock_info("anyone", &[]);
        let env = mock_env();
        instantiate(deps.as_mut(), env.clone(), info, InstantiateMsg {}).unwrap();
        // let balance = coins(100, "tokens");
        let balance1 = coin(100, "token");
        let balance2 = coin(100, "aside");
        let source_port =
            String::from("wasm.wasm1ghd753shjuwexxywmgs4xz7x2q732vcnkm6h2pyv9s6ah3hylvrq8epk7w");
        let source_channel = String::from("channel-9");
        let destination_channel = String::from("channel-10");
        let destination_port = String::from("swap");
        let sequence = 3;

        // Cannot create, no funds
        // let info = mock_info(&sender, &[]);
        let create = MakeSwapMsg {
            source_port: source_port.clone(),
            source_channel: source_channel.clone(),
            sell_token: balance1,
            buy_token: balance2,
            maker_address: "wasm1kj2t5txvwznrdx32v6xsw46yqztsyahqwxwlve".to_string(),
            maker_receiving_address: "wasm1kj2t5txvwznrdx32v6xsw46yqztsyahqwxwlve".to_string(),
            desired_taker: "".to_string(),
            expiration_timestamp: 1693399749000000000,
            timeout_height: Height {
                revision_number: 0,
                revision_height: 0,
            },
            timeout_timestamp: 1693399799000000000,
            take_bids: false
        };

        let path = order_path(
            source_channel.clone(),
            source_port.clone(),
            destination_channel.clone(),
            destination_port.clone(),
            //sequence.clone(),
        )
        .unwrap();

        let order_id = generate_order_id(&path);
        println!("order_id is {:?}", &order_id);
    }

    #[test]
    fn test_takeswap_msg_decode() {
        let mut deps = mock_dependencies();

        let info = mock_info("anyone", &[]);
        let env = mock_env();
        instantiate(deps.as_mut(), env.clone(), info, InstantiateMsg {}).unwrap();
        // let balance = coins(100, "tokens");
        let balance2 = coin(100, "aside");
        let taker_address = String::from("side1lqd386kze5355mgpncu5y52jcdhs85ckj7kdv0");
        let taker_receiving_address = String::from("wasm19zl4l2hafcdw6p99kc00znttgpdyk32a02puj2");

        let create = TakeSwapMsg {
            order_id: String::from(
                "bf4dd83fc04ea4bf565a0294ed15d189ee2d7662a1174428d3d46b46af55c7a2",
            ),
            sell_token: balance2,
            taker_address,
            taker_receiving_address,
            timeout_height: Height {
                revision_number: 0,
                revision_height: 0,
            },
            timeout_timestamp: 1693399799000000000,
        };

        let create_bytes = to_binary(&create.clone()).unwrap();
        println!("create_bytes is {:?}", &create_bytes.clone().to_base64());

        let bytes = Binary::from_base64("eyJvcmRlcl9pZCI6ImJmNGRkODNmYzA0ZWE0YmY1NjVhMDI5NGVkMTVkMTg5ZWUyZDc2NjJhMTE3NDQyOGQzZDQ2YjQ2YWY1NWM3YTIiLCJzZWxsX3Rva2VuIjp7ImRlbm9tIjoiYXNpZGUiLCJhbW91bnQiOiIxMDAifSwidGFrZXJfYWRkcmVzcyI6InNpZGUxbHFkMzg2a3plNTM1NW1ncG5jdTV5NTJqY2Roczg1Y2tqN2tkdjAiLCJ0YWtlcl9yZWNlaXZpbmdfYWRkcmVzcyI6Indhc20xOXpsNGwyaGFmY2R3NnA5OWtjMDB6bnR0Z3BkeWszMmEwMnB1ajIiLCJ0aW1lb3V0X2hlaWdodCI6eyJyZXZpc2lvbl9udW1iZXIiOiIwIiwicmV2aXNpb25faGVpZ2h0IjoiOTk5OTk5NiJ9LCJ0aW1lb3V0X3RpbWVzdGFtcCI6IjE2OTMzOTk3OTkwMDAwMDAwMDAiLCJjcmVhdGVfdGltZXN0YW1wIjoiMTY4NDMyODUyNyJ9").unwrap();

        println!("bytes is {:?}", &bytes.clone());
        // let msg: TakeSwapMsg = from_binary(&bytes.clone()).unwrap();

        let msg_res: Result<TakeSwapMsg, StdError> = from_binary(&bytes);
        let msg: TakeSwapMsg;

        match msg_res {
            Ok(value) => {
                msg = value.clone();
            }
            Err(_err) => {
                let msg_output: TakeSwapMsgOutput = from_binary(&bytes).unwrap();
                msg = TakeSwapMsg {
                    order_id: msg_output.order_id.clone(),
                    sell_token: msg_output.sell_token.clone(),
                    taker_address: msg_output.taker_address.clone(),
                    taker_receiving_address: msg_output.taker_receiving_address.clone(),
                    timeout_height: Height {
                        revision_number: msg_output
                            .timeout_height
                            .revision_number
                            .clone()
                            .parse()
                            .unwrap(),
                        revision_height: msg_output
                            .timeout_height
                            .revision_height
                            .clone()
                            .parse()
                            .unwrap(),
                    },
                    timeout_timestamp: msg_output.timeout_timestamp.parse().unwrap(),
                }
            }
        }
        println!("msg is {:?}", &msg);
    }
}
