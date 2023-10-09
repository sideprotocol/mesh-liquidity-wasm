use std::vec;

use cosmwasm_std::{Deps, Order, StdResult};
use cw_storage_plus::Bound;

use crate::{
    contract::{DEFAULT_LIMIT, MAX_LIMIT},
    msg::ListResponse,
    state::{AtomicSwapOrder, ORDER_TO_COUNT, SWAP_ORDERS},
};

pub fn query_list_reverse(
    deps: Deps,
    start_before: Option<u64>,
    limit: Option<u32>,
) -> StdResult<ListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let end;
    if start_before.is_some() {
        end = Some(Bound::exclusive(start_before.unwrap()));
    } else {
        end = None;
    }
    let swap_orders = SWAP_ORDERS
        .range(deps.storage, None, end, Order::Descending)
        .take(limit)
        .map(|item: Result<(u64, AtomicSwapOrder), cosmwasm_std::StdError>| item.unwrap().1)
        .collect::<Vec<AtomicSwapOrder>>();

    if swap_orders.is_empty() {
        return Ok(ListResponse {
            swaps: vec![],
            last_order_id: 0,
        });
    }
    let count_check = ORDER_TO_COUNT.load(deps.storage, &swap_orders.last().unwrap().id)?;
    Ok(ListResponse {
        swaps: swap_orders,
        last_order_id: count_check,
    })
}

pub fn query_list_by_desired_taker_reverse(
    deps: Deps,
    start_before: Option<u64>,
    limit: Option<u32>,
    desired_taker: String,
) -> StdResult<ListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let end;
    if start_before.is_some() {
        end = Some(Bound::exclusive(start_before.unwrap()));
    } else {
        end = None;
    }
    let swap_orders = SWAP_ORDERS
        .range(deps.storage, None, end, Order::Descending)
        .take(limit)
        .map(|item: Result<(u64, AtomicSwapOrder), cosmwasm_std::StdError>| item.unwrap().1)
        .filter(|swap_order| swap_order.maker.desired_taker == desired_taker)
        .collect::<Vec<AtomicSwapOrder>>();

    if swap_orders.is_empty() {
        return Ok(ListResponse {
            swaps: vec![],
            last_order_id: 0,
        });
    }
    let count_check = ORDER_TO_COUNT.load(deps.storage, &swap_orders.last().unwrap().id)?;
    Ok(ListResponse {
        swaps: swap_orders,
        last_order_id: count_check,
    })
}

pub fn query_list_by_maker_reverse(
    deps: Deps,
    start_before: Option<u64>,
    limit: Option<u32>,
    maker: String,
) -> StdResult<ListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let end;
    if start_before.is_some() {
        end = Some(Bound::exclusive(start_before.unwrap()));
    } else {
        end = None;
    }
    let swap_orders = SWAP_ORDERS
        .range(deps.storage, None, end, Order::Descending)
        .take(limit)
        .map(|item: Result<(u64, AtomicSwapOrder), cosmwasm_std::StdError>| item.unwrap().1)
        .filter(|swap_order| swap_order.maker.maker_address == maker)
        .collect::<Vec<AtomicSwapOrder>>();

    if swap_orders.is_empty() {
        return Ok(ListResponse {
            swaps: vec![],
            last_order_id: 0,
        });
    }
    let count_check = ORDER_TO_COUNT.load(deps.storage, &swap_orders.last().unwrap().id)?;
    Ok(ListResponse {
        swaps: swap_orders,
        last_order_id: count_check,
    })
}

pub fn query_list_by_taker_reverse(
    deps: Deps,
    start_before: Option<u64>,
    limit: Option<u32>,
    taker: String,
) -> StdResult<ListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let end;
    if start_before.is_some() {
        end = Some(Bound::exclusive(start_before.unwrap()));
    } else {
        end = None;
    }
    let swap_orders = SWAP_ORDERS
        .range(deps.storage, None, end, Order::Descending)
        .take(limit)
        .map(|item: Result<(u64, AtomicSwapOrder), cosmwasm_std::StdError>| item.unwrap().1)
        .filter(|swap_order| {
            swap_order.taker.is_some() && swap_order.taker.clone().unwrap().taker_address == taker
        })
        .collect::<Vec<AtomicSwapOrder>>();

    if swap_orders.is_empty() {
        return Ok(ListResponse {
            swaps: vec![],
            last_order_id: 0,
        });
    }
    let count_check = ORDER_TO_COUNT.load(deps.storage, &swap_orders.last().unwrap().id)?;
    Ok(ListResponse {
        swaps: swap_orders,
        last_order_id: count_check,
    })
}
