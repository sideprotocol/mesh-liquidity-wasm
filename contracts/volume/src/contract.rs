#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response,
    StdResult, StdError, Coin,
};

use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ ExecuteMsg, InstantiateMsg, MigrateMsg};
use crate::state::{
  CONFIG, OBSERVATIONS, Observation, Config,
};
use cw_storage_plus::Bound;

// Version info, for migration info
const CONTRACT_NAME: &str = "volume";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let config = Config {
        admin: info.sender.into_string(),
        contract_address: msg.contract,
        max_length: msg.max_length,
        pivoted: true,
        current_idx: 0,
        is_new: true
    };
    CONFIG.save(deps.storage, &config)?;
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
        ExecuteMsg::LogObservation { token1, token2 } => execute_log_observation(deps, env, info, token1, token2),
        ExecuteMsg::SetContract { address } => execute_set_contract(deps, env, info, address),
    }
}

pub fn execute_log_observation(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token1: Coin,
    token2: Coin
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.contract_address {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Must be called by contract"
        ))));
    }

    if config.is_new {
        let obs = Observation {
            block_timestamp: env.block.time.nanos(),
            volume1: token1.amount.u128(),
            volume2: token2.amount.u128(),
            num_of_observations: 1,
        };
        OBSERVATIONS.save(deps.storage, config.current_idx, &obs)?;
        config.is_new = false;
    } else {
        write(
            deps,
            env.block.time.nanos(),
            token1.amount.u128(),
            token2.amount.u128()
        )?;
    }
    CONFIG.save(deps.storage, &config)?;

    let res = Response::new()
        .add_attribute("action", "log_observation");
    Ok(res)
}

pub fn execute_set_contract(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    address: String
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Must be called by admin"
        ))));
    }

    config.contract_address = address;
    CONFIG.save(deps.storage, &config)?;

    let res = Response::new()
        .add_attribute("action", "set_contract");
    Ok(res)
}

/**
Writes an oracle observation to the struct.
Index represents the most recently written element.
Parameters:
+ `block_timestamp`: The timestamp (in nanoseconds) of the new observation.
+ `volume1`: volume of first token.
+ `volume2`: volume of second token.
*/
fn write(deps: DepsMut, block_timestamp: u64, volume1: u128, volume2: u128) -> Result<u64, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    let obs = OBSERVATIONS.load(deps.storage, config.current_idx)?;

    if block_timestamp == obs.block_timestamp {
        let new_obs = transform(&obs, block_timestamp, volume1, volume2);
        OBSERVATIONS.save(deps.storage, config.current_idx, &new_obs)?;
        return Ok(config.current_idx);
    }

    if config.current_idx + 1 >= config.max_length {
        config.pivoted = true;
        config.current_idx = 0;
    } else {
        config.current_idx += 1;
    }

    let new_obs = transform(&obs, block_timestamp, volume1, volume2);
    OBSERVATIONS.save(deps.storage, config.current_idx, &new_obs)?;

    CONFIG.save(deps.storage, &config)?;

    return Ok(config.current_idx);
}

/**
Transforms a previous observation into a new observation.
Parameters:
+ `block_timestamp`: _must_ be chronologically equal to or greater than last.block_timestamp.
+ `last`: The specified observation to be transformed.
+ `price1`: price of first token.
+ `price2`: price of second token.
*/
pub fn transform(
    last: &Observation,
    block_timestamp: u64,
    volume1: u128,
    volume2: u128,
) -> Observation {
    return Observation {
        block_timestamp: block_timestamp,
        num_of_observations: last.num_of_observations + 1,
        volume1: last.volume1 + volume1,
        volume2: last.volume2 + volume2,
    };
}

/**
Pivoted point binary search: searches array which is
sorted and rotated from a particular point.
Similar to rotated array from a certain pivot point.
Parameters:
+ `block_timestamp`: timestamp in nanoseconds.
*/
pub fn binary_search(deps: Deps, block_timestamp: u64) -> StdResult<u64> {
    let config = CONFIG.load(deps.storage)?;
    // edge case when all values are less than required
    let obs = OBSERVATIONS.load(deps.storage, config.current_idx)?;
    if obs.block_timestamp < block_timestamp
    {
        panic!("Observation after this timestamp doesn't exist");
    }

    let mut start: u64 = 0;
    let mut end: u64 = config.current_idx + 1;
    let mut mid: u64;

    while start < end {
        mid = (start + end) / 2;
        if block_timestamp <= OBSERVATIONS.load(deps.storage, mid)?.block_timestamp {
            end = mid;
        } else {
            start = mid + 1;
        }
    }

    if config.pivoted && start == 0 {
        let res = start;
        start = config.current_idx + 1;
        end = observations.len();

        while start < end {
            mid = (start + end) / 2;
            if block_timestamp <= OBSERVATIONS.load(deps.storage, mid)?.block_timestamp {
                end = mid;
            } else {
                start = mid + 1;
            }
        }
        if start >= observations.len() {
            start = res;
        }
    }

    return Ok(start);
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

