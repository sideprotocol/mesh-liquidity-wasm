use std::cmp::min;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
};

use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{Config, VestingDetails, CONFIG, VESTED_TOKENS_ALL};

// Version info, for migration info
const CONTRACT_NAME: &str = "vesting";
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
        allowed_addresses: msg.allowed_addresses,
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
        ExecuteMsg::StartVesting { vesting } => execute_start_vesting(deps, env, info, vesting),
        ExecuteMsg::SetAllowed { addresses } => execute_set_contract(deps, env, info, addresses),
        ExecuteMsg::Claim {} => execute_claim(deps, env, info),
    }
}

pub fn execute_start_vesting(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    vesting: VestingDetails,
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
        CONFIG.save(deps.storage, &config)?;
        OBSERVATIONS.save(deps.storage, config.current_idx, &obs)?;
        config.is_new = false;
    } else {
        write(
            deps,
            env.block.time.nanos(),
            token1.amount.u128(),
            token2.amount.u128(),
        )?;
    }

    let res = Response::new().add_attribute("action", "log_observation");
    Ok(res)
}

pub fn execute_set_contract(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    addresses: Vec<String>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Must be called by admin"
        ))));
    }

    config.contract_address = address;
    CONFIG.save(deps.storage, &config)?;

    let res = Response::new().add_attribute("action", "set_contract");
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
fn write(
    deps: DepsMut,
    block_timestamp: u64,
    volume1: u128,
    volume2: u128,
) -> Result<u64, ContractError> {
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
    config.counter += 1;

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
    if obs.block_timestamp < block_timestamp {
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
        end = min(config.max_length, config.counter);

        while start < end {
            mid = (start + end) / 2;
            if block_timestamp <= OBSERVATIONS.load(deps.storage, mid)?.block_timestamp {
                end = mid;
            } else {
                start = mid + 1;
            }
        }
        if start >= min(config.max_length, config.counter) {
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
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Contract {} => to_binary(&query_contract(deps)?),
        QueryMsg::TotalVolume {} => to_binary(&query_total_volume(deps, env)?),
        QueryMsg::TotalVolumeAt { timestamp } => {
            to_binary(&query_total_volume_at(deps, timestamp)?)
        } //QueryMsg::VolumeInterval { start, end } => to_binary(&query_total_volume_interval(deps, start, end)?),
    }
}

fn query_contract(deps: Deps) -> StdResult<String> {
    let config = CONFIG.load(deps.storage)?;

    Ok(config.contract_address)
}

fn query_total_volume(deps: Deps, env: Env) -> StdResult<Observation> {
    let res = binary_search(deps, env.block.time.nanos())?;
    Ok(OBSERVATIONS.load(deps.storage, res)?)
}

fn query_total_volume_at(deps: Deps, timestamp: u64) -> StdResult<Observation> {
    let res = binary_search(deps, timestamp)?;
    Ok(OBSERVATIONS.load(deps.storage, res)?)
}

// fn query_total_volume_interval(
//     deps: Deps,
//     start: u64,
//     end: u64
// ) -> StdResult<Observation> {
//     let res = binary_search(deps, timestamp)?;
//     Ok(OBSERVATIONS.load(deps.storage, res)?)
// }
