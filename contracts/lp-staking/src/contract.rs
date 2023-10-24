use std::collections::HashSet;

use cosmwasm_std::{
    entry_point, from_binary, Addr, Api, Decimal, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, Uint128, Uint64,
};

use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;

use crate::error::ContractError;
use crate::msg::{Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg};
use crate::state::{Config, PoolInfo, CONFIG, POOL_INFO};

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
        ExecuteMsg::SetTokensPerBlock { amount } => execute_set_tokens_per_block(deps, env, amount),
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

/// Sets a new amount of veToken distributed per block among all active pools. Before that, we
/// will need to update all pools in order to correctly account for accured rewards.
///
/// * **amount** new count of tokens per block.
fn execute_set_tokens_per_block(
    mut deps: DepsMut,
    env: Env,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let mut cfg = CONFIG.load(deps.storage)?;

    let pools: Vec<_> = cfg.active_pools.iter().map(|pool| pool.0.clone()).collect();

    update_pools(deps.branch(), &env, &cfg, &pools)?;

    cfg.tokens_per_block = amount;
    CONFIG.save(deps.storage, &cfg)?;

    Ok(Response::new().add_attribute("action", "set_tokens_per_block"))
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
        Cw20HookMsg::Deposit {} => {
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
        accumulate_rewards_per_share(env, lp_token, &mut pool, cfg)?;
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

pub fn create_pool(
    deps: DepsMut,
    env: &Env,
    lp_token: &Addr,
    cfg: &Config,
) -> Result<(), ContractError> {
    POOL_INFO.save(
        deps.storage,
        &lp_token,
        &PoolInfo {
            last_reward_block: cfg
                .start_block
                .max(Uint64::from(env.block.height).into())
                .into(),
            has_asset_rewards: false,
            reward_global_index: Decimal::zero(),
            total_virtual_supply: Default::default(),
        },
    )?;

    Ok(())
}

/// Calculates and returns the amount of accured rewards since the last reward checkpoint for a specific lp-token.
///
/// * **alloc_point** allocation points for specific lp-token.
pub fn calculate_rewards(n_blocks: u64, alloc_point: &Uint128, cfg: &Config) -> StdResult<Uint128> {
    let r = Uint128::from(n_blocks)
        .checked_mul(cfg.tokens_per_block.into())?
        .checked_mul(*alloc_point)?
        .checked_div(cfg.total_alloc_point.into())
        .unwrap_or_else(|_| Uint128::zero());

    Ok(r)
}

/// Gets allocation point of the pool.
///
/// * **pools** is a vector of set that contains LP token address and allocation point.
pub fn get_alloc_point(pools: &Vec<(Addr, Uint128)>, lp_token: &Addr) -> Uint128 {
    pools
        .iter()
        .find_map(|(addr, alloc_point)| {
            if &addr == lp_token {
                return Some(*alloc_point);
            }
            None
        })
        .unwrap_or_else(Uint128::zero)
}

/// Accures the amount of rewards distributed for each staked LP token.
/// Also update reward variables for the given lp-token.
///
/// * **lp_token** LP token whose rewards per share we update.
///
/// * **pool** generator associated with the `lp_token`.
pub fn accumulate_rewards_per_share(
    env: &Env,
    lp_token: &Addr,
    pool: &mut PoolInfo,
    cfg: &Config,
) -> StdResult<()> {
    // we should calculate rewards by previous virtual amount
    let lp_supply = pool.total_virtual_supply;

    if env.block.height > pool.last_reward_block.u64() {
        if !lp_supply.is_zero() {
            let alloc_point = get_alloc_point(&cfg.active_pools, &lp_token);

            let token_rewards = calculate_rewards(
                env.block.height - pool.last_reward_block.u64(),
                &alloc_point,
                cfg,
            )?;

            let share = Decimal::from_ratio(token_rewards, lp_supply);
            pool.reward_global_index = pool.reward_global_index.checked_add(share)?;
        }

        pool.last_reward_block = Uint64::from(env.block.height);
    }

    Ok(())
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
