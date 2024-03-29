use std::collections::HashSet;

use cosmwasm_std::{
    entry_point, from_binary, to_binary, Addr, Api, Decimal, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, Uint128, Uint64, WasmMsg,
};

use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::decimal_checked_ops::DecimalCheckedOps;
use crate::error::ContractError;
use crate::msg::{Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg};
use crate::state::{Config, PoolInfo, UserInfo, CONFIG, POOL_INFO, USER_INFO};

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

    let reward = addr_validate_to_lower(deps.api, &msg.reward_token)?;

    let config = Config {
        admin: info.sender,
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
        ExecuteMsg::ClaimRewards {
            lp_tokens,
            receiver,
        } => execute_claim_rewards(deps, env, lp_tokens, receiver),
        ExecuteMsg::UpdateConfig { reward_token } => {
            execute_update_config(deps, info, reward_token)
        }
        ExecuteMsg::Withdraw { lp_token, amount } => {
            execute_withdraw(deps, env, lp_token, info.sender, amount)
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

/// Updates the amount of accured rewards for a specific lp-token.
///
/// * **lp_token** sets the liquidity pool to be updated and claimed.
///
/// * **account** receiver address.
pub fn execute_claim_rewards(
    mut deps: DepsMut,
    env: Env,
    lp_tokens_raw: Vec<String>,
    account: String,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;

    let mut lp_tokens = vec![];
    for lp_token in &lp_tokens_raw {
        lp_tokens.push(addr_validate_to_lower(deps.api, lp_token)?);
    }
    let account = addr_validate_to_lower(deps.api, account)?;

    update_pools(deps.branch(), &env, &cfg, &lp_tokens)?;

    let mut send_rewards_msg = vec![];
    for lp_token in &lp_tokens {
        let pool = POOL_INFO.load(deps.storage, &lp_token.clone()).unwrap();
        let mut user = USER_INFO
            .load(deps.storage, &(lp_token.clone(), account.clone()))
            .unwrap();

        send_rewards_msg.append(&mut send_pending_rewards(&cfg, &pool, &user, &account)?);

        // Update user's reward index
        user.reward_user_index = pool.reward_global_index;

        USER_INFO.save(deps.storage, &(lp_token.clone(), account.clone()), &user)?;
        POOL_INFO.save(deps.storage, &lp_token, &pool)?;
    }

    Ok(Response::default()
        .add_attribute("action", "claim_rewards")
        .add_messages(send_rewards_msg))
}

/// ## Executor
/// Only the owner can execute this.
#[allow(clippy::too_many_arguments)]
pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    reward_token: String,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    // Permission check
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    config.reward_token = addr_validate_to_lower(deps.api, reward_token)?;

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

/// Withdraw LP tokens from contract.
///
/// * **lp_token** LP token to withdraw.
///
/// * **account** user whose LP tokens we withdraw.
///
/// * **amount** amount of LP tokens to withdraw.
pub fn execute_withdraw(
    deps: DepsMut,
    env: Env,
    lp_token: String,
    account: Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let lp_token = addr_validate_to_lower(deps.api, lp_token)?;
    let mut user = USER_INFO
        .load(deps.storage, &(lp_token.clone(), account.clone()))
        .unwrap_or_default();
    if user.amount < amount {
        return Err(ContractError::BalanceTooSmall {});
    }

    let cfg = CONFIG.load(deps.storage)?;
    let mut pool = POOL_INFO.load(deps.storage, &lp_token.clone()).unwrap();

    accumulate_rewards_per_share(&env, &lp_token.clone(), &mut pool, &cfg)?;

    // Send pending rewards to the user
    let send_rewards_msg = send_pending_rewards(&cfg, &pool, &user, &account)?;

    // Instantiate the transfer call for the LP token
    let transfer_msg = if !amount.is_zero() {
        vec![WasmMsg::Execute {
            contract_addr: lp_token.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: account.to_string(),
                amount: amount,
            })?,
            funds: vec![],
        }]
    } else {
        vec![]
    };

    // Update user's balance
    user.amount = user.amount.checked_sub(amount).unwrap();
    user.reward_user_index = pool.reward_global_index;
    pool.total_supply -= user.amount;

    POOL_INFO.save(deps.storage, &lp_token, &pool)?;

    if !user.amount.is_zero() {
        USER_INFO.save(deps.storage, &(lp_token, account), &user)?;
    } else {
        USER_INFO.remove(deps.storage, &(lp_token, account));
    }

    Ok(Response::new()
        .add_messages(send_rewards_msg)
        .add_messages(transfer_msg)
        .add_attribute("action", "withdraw")
        .add_attribute("amount", amount))
}

/// Receives a message of type [`Cw20ReceiveMsg`] and processes it depending on the received template.
/// * **cw20** CW20 message to process.
fn receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let amount = cw20_msg.amount;
    let lp_token = info.sender;

    match from_binary(&cw20_msg.msg)? {
        Cw20HookMsg::Deposit {} => {
            let account = addr_validate_to_lower(deps.api, &cw20_msg.sender)?;
            if !POOL_INFO.has(deps.storage, &lp_token) {
                return Err(ContractError::PoolNotFound {});
            }

            deposit(deps, env, lp_token, account, amount)
        }
        Cw20HookMsg::DepositFor { beneficiary } => {
            deposit(deps, env, lp_token, beneficiary, amount)
        }
    }
}

/// Deposit LP tokens in contract to receive token emissions.
///
/// * **lp_token** LP token to deposit.
///
/// * **beneficiary** address that will take ownership of the staked LP tokens.
///
/// * **amount** amount of LP tokens to deposit.
pub fn deposit(
    deps: DepsMut,
    env: Env,
    lp_token: Addr,
    beneficiary: Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let mut user = USER_INFO
        .load(deps.storage, &(lp_token.clone(), beneficiary.clone()))
        .unwrap_or_default();

    let cfg = CONFIG.load(deps.storage)?;
    let mut pool = POOL_INFO.load(deps.storage, &lp_token)?;

    accumulate_rewards_per_share(&env, &lp_token, &mut pool, &cfg)?;

    // Send pending rewards (if any) to the depositor
    let send_rewards_msg = send_pending_rewards(&cfg, &pool, &user, &beneficiary.clone())?;

    // Update user's LP token balance
    user.amount = user.amount.checked_add(amount).unwrap();
    user.reward_user_index = pool.reward_global_index;
    pool.total_supply += user.amount;

    POOL_INFO.save(deps.storage, &lp_token, &pool)?;
    USER_INFO.save(deps.storage, &(lp_token, beneficiary), &user)?;

    Ok(Response::new()
        .add_messages(send_rewards_msg)
        .add_attribute("action", "deposit")
        .add_attribute("amount", amount))
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
            total_supply: Default::default(),
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
    let lp_supply = pool.total_supply;
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

/// Distributes pending rewards for a specific staker.
///
/// * **pool** lp token where user staked.
///
/// * **user** staker for which we claim rewards.
///
/// * **to** address that will receive rewards.
pub fn send_pending_rewards(
    cfg: &Config,
    pool: &PoolInfo,
    user: &UserInfo,
    to: &Addr,
) -> Result<Vec<WasmMsg>, ContractError> {
    if user.amount.is_zero() {
        return Ok(vec![]);
    }

    let mut messages = vec![];

    let pending_rewards = (pool.reward_global_index - user.reward_user_index)
        .checked_mul_uint128(user.amount)
        .unwrap();

    if !pending_rewards.is_zero() {
        messages.push(WasmMsg::Execute {
            contract_addr: cfg.reward_token.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: to.to_string(),
                amount: pending_rewards,
            })?,
            funds: vec![],
        });
    }

    Ok(messages)
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
