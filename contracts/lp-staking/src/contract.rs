use std::collections::HashSet;

use cosmwasm_std::{
    entry_point, to_binary, Addr, Api, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, Uint128,
};

use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{Config, Observation, CONFIG, OBSERVATIONS, POOL_INFO};

// Version info, for migration info
const CONTRACT_NAME: &str = "lp-staking";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let deposit = addr_validate_to_lower(deps.api, &msg.deposit_token)?;
    let reward = addr_validate_to_lower(deps.api, &msg.reward_token)?;

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

/// Creates a new reward emitter and adds it to [`POOL_INFO`] (if it does not exist yet) and updates
/// total allocation points (in [`Config`]).
///
/// * **pools** is a vector of set that contains LP token address and allocation point.
///
/// ## Executor
/// Can only be called by the owner or reward emitter controller
pub fn execute_setup_pools(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pools: Vec<(String, Uint128)>,
) -> Result<Response, ContractError> {
    let mut cfg = CONFIG.load(deps.storage)?;
    if info.sender != cfg.admin {
        return Err(ContractError::Unauthorized {});
    }

    let pools_set: HashSet<String> = pools.clone().into_iter().map(|pc| pc.0).collect();
    if pools_set.len() != pools.len() {
        return Err(ContractError::PoolDuplicate {});
    }

    let mut setup_pools: Vec<(Addr, Uint128)> = vec![];

    for (pool, alloc_point) in pools {
        let pool_addr = addr_validate_to_lower(deps.api, &pool)?;
        setup_pools.push((pool_addr, alloc_point));
    }

    let prev_pools: Vec<_> = cfg.active_pools.iter().map(|pool| pool.0.clone()).collect();

    update_pools(deps.branch(), &env, &cfg, &prev_pools)?;

    for (lp_token, _) in &setup_pools {
        if !POOL_INFO.has(deps.storage, &lp_token) {
            create_pool(deps.branch(), &env, &lp_token, &cfg)?;
        }
    }

    cfg.total_alloc_point = setup_pools.iter().map(|(_, alloc_point)| alloc_point).sum();
    cfg.active_pools = setup_pools;

    CONFIG.save(deps.storage, &cfg)?;

    Ok(Response::new().add_attribute("action", "setup_pools"))
}

/// Updates the amount of accrued rewards.
///
/// * **lp_tokens** is the list of LP tokens which should be updated.
pub fn update_pools(
    deps: DepsMut,
    env: &Env,
    cfg: &Config,
    lp_tokens: &Vec<Addr>,
) -> Result<(), ContractError> {
    for lp_token in lp_tokens {
        let mut pool = POOL_INFO.load(deps.storage, &lp_token)?;
        accumulate_rewards_per_share(&deps.querier, env, lp_token, &mut pool, cfg)?;
        POOL_INFO.save(deps.storage, lp_token, &pool)?;
    }

    Ok(())
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

/// Returns a lowercased, validated address upon success. Otherwise returns [`Err`]
/// ## Params
/// * **api** is an object of type [`Api`]
///
/// * **addr** is an object of type [`impl Into<String>`]
pub fn addr_validate_to_lower(api: &dyn Api, addr: impl Into<String>) -> StdResult<Addr> {
    let addr = addr.into();
    if addr.to_lowercase() != addr {
        return Err(StdError::generic_err(format!(
            "Address {} should be lowercase",
            addr
        )));
    }
    api.addr_validate(&addr)
}

// fn query_total_volume_interval(
//     deps: Deps,
//     start: u64,
//     end: u64
// ) -> StdResult<Observation> {
//     let res = binary_search(deps, timestamp)?;
//     Ok(OBSERVATIONS.load(deps.storage, res)?)
// }
