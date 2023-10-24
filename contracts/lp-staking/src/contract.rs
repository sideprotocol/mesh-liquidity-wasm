use cosmwasm_std::{
    entry_point, to_binary, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult,
};

use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{Config, Observation, CONFIG, OBSERVATIONS};

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

    let deposit = deps.api.addr_validate(&msg.deposit_token)?;
    let reward = deps.api.addr_validate(&msg.reward_token)?;

    let config = Config {
        admin: info.sender,
        deposit_token: deposit,
        tokens_per_block: msg.tokens_per_block,
        total_alloc_point: msg.total_alloc_point,
        start_block: msg.start_block,
        reward_token: reward,
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
        ExecuteMsg::SetupPools { pools } => execute_setup_pools(deps, env, info, pools),
        ExecuteMsg::SetTokensPerBlock { amount } => {
            execute_set_tokens_per_block(deps, env, info, amount)
        }
        ExecuteMsg::ClaimRewards { lp_tokens } => execute_claim_rewards(deps, env, info, lp_tokens),
    }
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
