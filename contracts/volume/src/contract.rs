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
    // CHANNEL_INFO,
    SWAP_ORDERS, set_atomic_order, get_atomic_order, COUNT, move_order_to_bottom, BID_ORDER_TO_COUNT, Bid, BIDS,
};
use cw_storage_plus::Bound;

// Version info, for migration info
const CONTRACT_NAME: &str = "volume";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

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

    let ibc_packet = AtomicSwapPacketData {
        r#type: SwapMessageType::MakeSwap,
        data: to_binary(&msg)?,
        order_id: None,
        path: None,
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
        QueryMsg::List { start_after, limit } => to_binary(&query_list(deps, start_after, limit)?),
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
) -> StdResult<ListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|s| Bound::ExclusiveRaw(s.into_bytes()));
    let swap_orders = SWAP_ORDERS
        .range(deps.storage, start, None, Order::Ascending)
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

