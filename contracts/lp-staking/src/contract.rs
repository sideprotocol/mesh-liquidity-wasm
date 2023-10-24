use std::collections::HashSet;

use cosmwasm_std::{
    entry_point, from_binary, Addr, Api, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
    Uint128,
};

use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg};
use crate::state::{Config, CONFIG, POOL_INFO};

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
        active_pools: vec![],
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
        ExecuteMsg::UpdateConfig {} => execute_update_config(deps, env, info),
        ExecuteMsg::Withdraw { lp_token, amount } => {
            execute_withdraw(deps, env, info, lp_token, amount)
        }
        ExecuteMsg::Receive(msg) => receive(deps, env, info, msg),
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

/// Receives a message of type [`Cw20ReceiveMsg`] and processes it depending on the received template.
/// * **cw20** CW20 message to process.
fn receive(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let amount = cw20_msg.amount;
    let lp_token = info.sender;
    let cfg = CONFIG.load(deps.storage)?;

    match from_binary(&cw20_msg.msg)? {
        Cw20HookMsg::Deposit { token_code_hash } => {
            let account = addr_validate_to_lower(deps.api, &cw20_msg.sender)?;
            if !POOL_INFO.has(deps.storage, &lp_token) {
                create_pool(deps.branch(), &env, &lp_token, &cfg)?;
            }

            deposit(deps, env, lp_token, account, amount)
        }
        Cw20HookMsg::DepositFor { beneficiary } => {
            deposit(deps, env, lp_token, beneficiary, amount)
        }
    }
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
