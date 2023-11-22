#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, Storage, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{BalanceResponse, Cw20ExecuteMsg, Cw20ReceiveMsg, TokenInfoResponse};
use cw20_base::state::{MinterData, TokenInfo, TOKEN_INFO};

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, LockInfoResponse, MigrateMsg,
    QueryMsg, VotingPowerResponse,
};
use crate::state::{Config, Lock, Point, CONFIG, HISTORY, LAST_SLOPE_CHANGE, LOCKED};
use crate::utils::{
    adjust_vp_and_slope, calc_coefficient, calc_voting_power, cancel_scheduled_slope,
    fetch_last_checkpoint, fetch_slope_changes, get_period, get_periods_count,
    schedule_slope_change, time_limits_check, EPOCH_START, WEEK,
};
use crate::DecimalCheckedOps;

/// Contract name that is used for migration.
const CONTRACT_NAME: &str = "ve-side";
/// Contract version that is used for migration.
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Creates a new contract with the specified parameters in [`InstantiateMsg`].
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let deposit_token = deps.api.addr_validate(&msg.deposit_token)?;

    let config = Config {
        admin: deps.api.addr_validate(&msg.admin)?,
        deposit_token,
    };
    CONFIG.save(deps.storage, &config)?;

    let cur_period = get_period(env.block.time.seconds())?;
    let point = Point {
        power: Uint128::zero(),
        start: cur_period,
        end: 0,
        slope: Default::default(),
    };
    HISTORY.save(
        deps.storage,
        (env.contract.address.clone(), cur_period),
        &point,
    )?;

    // Store token info
    let data = TokenInfo {
        name: "Vote Escrowed SIDE".to_string(),
        symbol: "veSIDE".to_string(),
        decimals: 6,
        total_supply: Uint128::zero(),
        mint: Some(MinterData {
            minter: env.contract.address,
            cap: None,
        }),
    };

    TOKEN_INFO.save(deps.storage, &data)?;

    Ok(Response::default())
}

/// Exposes all the execute functions available in the contract.
///
/// ## Execute messages
/// * **ExecuteMsg::ExtendLockTime { time }** Increase a staker's lock time.
///
/// * **ExecuteMsg::Receive(msg)** Parse incoming messages coming from the LP token contract.
///
/// * **ExecuteMsg::Withdraw {}** Withdraw all LP from a lock position if the lock has expired.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ExtendLockTime { time } => extend_lock_time(deps, env, info, time),
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::Withdraw {} => withdraw(deps, env, info),
    }
}

/// Checkpoint the total voting power (total supply of veSIDE).
/// This function fetches the last available veSIDE checkpoint, recalculates passed periods since the checkpoint and until now,
/// applies slope changes and saves all recalculated periods in [`HISTORY`].
///
/// * **add_voting_power** amount of veSIDE to add to the total.
///
/// * **reduce_power** amount of veSIDE to subtract from the total.
///
/// * **old_slope** old slope applied to the total voting power (veSIDE supply).
///
/// * **new_slope** new slope to be applied to the total voting power (veSIDE supply).
fn checkpoint_total(
    storage: &mut dyn Storage,
    env: Env,
    add_voting_power: Option<Uint128>,
    reduce_power: Option<Uint128>,
    old_slope: Uint128,
    new_slope: Uint128,
) -> StdResult<()> {
    let cur_period = get_period(env.block.time.seconds())?;
    let cur_period_key = cur_period;
    let contract_addr = env.contract.address;
    let add_voting_power = add_voting_power.unwrap_or_default();

    // Get last checkpoint
    let last_checkpoint = fetch_last_checkpoint(storage, &contract_addr, cur_period_key)?;
    let new_point = if let Some((_, mut point)) = last_checkpoint {
        let last_slope_change = LAST_SLOPE_CHANGE.may_load(storage)?.unwrap_or(0);
        if last_slope_change < cur_period {
            let scheduled_slope_changes =
                fetch_slope_changes(storage, last_slope_change, cur_period)?;
            // Recalculating passed points
            for (recalc_period, scheduled_change) in scheduled_slope_changes {
                point = Point {
                    power: calc_voting_power(&point, recalc_period),
                    start: recalc_period,
                    slope: point.slope - scheduled_change,
                    ..point
                };
                HISTORY.save(storage, (contract_addr.clone(), recalc_period), &point)?
            }

            LAST_SLOPE_CHANGE.save(storage, &cur_period)?
        }

        let new_power = (calc_voting_power(&point, cur_period) + add_voting_power)
            .saturating_sub(reduce_power.unwrap_or_default());

        Point {
            power: new_power,
            slope: point.slope - old_slope + new_slope,
            start: cur_period,
            ..point
        }
    } else {
        Point {
            power: add_voting_power,
            slope: new_slope,
            start: cur_period,
            end: 0, // we don't use 'end' in total voting power calculations
        }
    };
    HISTORY.save(storage, (contract_addr, cur_period_key), &new_point)
}

/// Checkpoint a user's voting power (veSIDE balance).
/// This function fetches the user's last available checkpoint, calculates the user's current voting power, applies slope changes based on
/// `add_amount` and `new_end` parameters, schedules slope changes for total voting power and saves the new checkpoint for the current
/// period in [`HISTORY`] (using the user's address).
/// If a user already checkpointed themselves for the current period, then this function uses the current checkpoint as the latest
/// available one.
///
/// * **addr** staker for which we checkpoint the voting power.
///
/// * **add_amount** amount of veSIDE to add to the staker's balance.
///
/// * **new_end** new lock time for the staker's veSIDE position.
fn checkpoint(
    deps: DepsMut,
    env: Env,
    addr: Addr,
    add_amount: Option<Uint128>,
    new_end: Option<u64>,
) -> StdResult<()> {
    let cur_period = get_period(env.block.time.seconds())?;
    let cur_period_key = cur_period;
    let add_amount = add_amount.unwrap_or_default();
    let mut old_slope = Default::default();
    let mut add_voting_power = Uint128::zero();

    // Get the last user checkpoint
    let last_checkpoint = fetch_last_checkpoint(deps.storage, &addr, cur_period_key)?;
    let new_point = if let Some((_, point)) = last_checkpoint {
        let end = new_end.unwrap_or(point.end);
        let dt = end.saturating_sub(cur_period);
        let current_power = calc_voting_power(&point, cur_period);
        let new_slope = if dt != 0 {
            if end > point.end && add_amount.is_zero() {
                // This is extend_lock_time. Recalculating user's voting power
                let mut lock = LOCKED.load(deps.storage, addr.clone())?;
                let mut new_voting_power = calc_coefficient(dt).checked_mul_uint128(lock.amount)?;
                let slope = adjust_vp_and_slope(&mut new_voting_power, dt)?;
                // new_voting_power should always be >= current_power. saturating_sub is used for extra safety
                add_voting_power = new_voting_power.saturating_sub(current_power);
                lock.last_extend_lock_period = cur_period;
                LOCKED.save(deps.storage, addr.clone(), &lock, env.block.height)?;
                slope
            } else {
                // This is an increase in the user's lock amount
                let raw_add_voting_power = calc_coefficient(dt).checked_mul_uint128(add_amount)?;
                let mut new_voting_power = current_power.checked_add(raw_add_voting_power)?;
                let slope = adjust_vp_and_slope(&mut new_voting_power, dt)?;
                // new_voting_power should always be >= current_power. saturating_sub is used for extra safety
                add_voting_power = new_voting_power.saturating_sub(current_power);
                slope
            }
        } else {
            Uint128::zero()
        };

        // Cancel the previously scheduled slope change
        cancel_scheduled_slope(deps.storage, point.slope, point.end)?;

        // We need to subtract the slope point from the total voting power slope
        old_slope = point.slope;

        Point {
            power: current_power + add_voting_power,
            slope: new_slope,
            start: cur_period,
            end,
        }
    } else {
        // This error can't happen since this if-branch is intended for checkpoint creation
        let end =
            new_end.ok_or_else(|| StdError::generic_err("Checkpoint initialization error"))?;
        let dt = end - cur_period;
        add_voting_power = calc_coefficient(dt).checked_mul_uint128(add_amount)?;
        let slope = adjust_vp_and_slope(&mut add_voting_power, dt)?;
        Point {
            power: add_voting_power,
            slope,
            start: cur_period,
            end,
        }
    };

    // Schedule a slope change
    schedule_slope_change(deps.storage, new_point.slope, new_point.end)?;

    HISTORY.save(deps.storage, (addr, cur_period_key), &new_point)?;
    checkpoint_total(
        deps.storage,
        env,
        Some(add_voting_power),
        None,
        old_slope,
        new_point.slope,
    )
}

/// Receives a message of type [`Cw20ReceiveMsg`] and processes it depending on the received template.
///
/// * **cw20_msg** CW20 message to process.
fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let sender = Addr::unchecked(cw20_msg.sender);
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.deposit_token {
        return Err(ContractError::Unauthorized {});
    }

    match from_binary(&cw20_msg.msg)? {
        Cw20HookMsg::CreateLock { time } => create_lock(deps, env, sender, cw20_msg.amount, time),
        Cw20HookMsg::ExtendLockAmount {} => deposit_for(deps, env, cw20_msg.amount, sender),
        Cw20HookMsg::DepositFor { user } => {
            let addr = deps.api.addr_validate(&user)?;
            deposit_for(deps, env, cw20_msg.amount, addr)
        }
    }
}

/// Creates a lock for the user that lasts for the specified time duration (in seconds).
/// Checks that the user is locking LP tokens.
/// Checks that the lock time is within [`WEEK`]..[`MAX_LOCK_TIME`].
/// Creates a lock if it doesn't exist and triggers a [`checkpoint`] for the staker.
/// If a lock already exists, then a [`ContractError`] is returned.
///
/// * **user** staker for which we create a lock position.
///
/// * **amount** amount of LP deposited in the lock position.
///
/// * **time** duration of the lock.
fn create_lock(
    deps: DepsMut,
    env: Env,
    user: Addr,
    amount: Uint128,
    time: u64,
) -> Result<Response, ContractError> {
    time_limits_check(time)?;

    let block_period = get_period(env.block.time.seconds())?;
    let end = block_period + get_periods_count(time);

    LOCKED.update(deps.storage, user.clone(), env.block.height, |lock_opt| {
        if lock_opt.is_some() && !lock_opt.unwrap().amount.is_zero() {
            return Err(ContractError::LockAlreadyExists {});
        }
        Ok(Lock {
            amount,
            start: block_period,
            end,
            last_extend_lock_period: block_period,
        })
    })?;

    checkpoint(deps, env, user, Some(amount), Some(end))?;

    Ok(Response::default().add_attribute("action", "create_lock"))
}

/// Deposits an 'amount' of LP tokens into 'user''s lock.
/// Checks that the user is transferring and locking LP.
/// Triggers a [`checkpoint`] for the user.
/// If the user does not have a lock, then a [`ContractError`] is returned.
///
/// * **amount** amount of LP to deposit.
///
/// * **user** user who's lock amount will increase.
fn deposit_for(
    deps: DepsMut,
    env: Env,
    amount: Uint128,
    user: Addr,
) -> Result<Response, ContractError> {
    LOCKED.update(
        deps.storage,
        user.clone(),
        env.block.height,
        |lock_opt| match lock_opt {
            Some(mut lock) if !lock.amount.is_zero() => {
                if lock.end <= get_period(env.block.time.seconds())? {
                    Err(ContractError::LockExpired {})
                } else {
                    lock.amount += amount;
                    Ok(lock)
                }
            }
            _ => Err(ContractError::LockDoesNotExist {}),
        },
    )?;
    checkpoint(deps, env, user, Some(amount), None)?;

    Ok(Response::default().add_attribute("action", "deposit_for"))
}

/// Withdraws the whole amount of locked LP from a specific user lock.
/// If the user lock doesn't exist or if it has not yet expired, then a [`ContractError`] is returned.
fn withdraw(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let sender = info.sender;
    // 'LockDoesNotExist' is thrown either when a lock does not exist in LOCKED or when a lock exists but lock.amount == 0
    let mut lock = LOCKED
        .may_load(deps.storage, sender.clone())?
        .filter(|lock| !lock.amount.is_zero())
        .ok_or(ContractError::LockDoesNotExist {})?;

    let cur_period = get_period(env.block.time.seconds())?;
    if lock.end > cur_period {
        Err(ContractError::LockHasNotExpired {})
    } else {
        let config = CONFIG.load(deps.storage)?;
        let transfer_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.deposit_token.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: sender.to_string(),
                amount: lock.amount,
            })?,
            funds: vec![],
        });
        lock.amount = Uint128::zero();
        LOCKED.save(deps.storage, sender.clone(), &lock, env.block.height)?;

        // We need to checkpoint and eliminate the slope influence on a future lock
        HISTORY.save(
            deps.storage,
            (sender, cur_period),
            &Point {
                power: Uint128::zero(),
                start: cur_period,
                end: cur_period,
                slope: Default::default(),
            },
        )?;

        Ok(Response::default()
            .add_message(transfer_msg)
            .add_attribute("action", "withdraw"))
    }
}

/// Increase the current lock time for a staker by a specified time period.
/// Evaluates that the `time` is within [`WEEK`]..[`MAX_LOCK_TIME`]
/// and then it triggers a [`checkpoint`].
/// If the user lock doesn't exist or if it expired, then a [`ContractError`] is returned.
///
/// ## Note
/// The time is added to the lock's `end`.
/// For example, at period 0, the user has their LP locked for 3 weeks.
/// In 1 week, they increase their lock time by 10 weeks, thus the unlock period becomes 13 weeks.
///
/// * **time** increase in lock time applied to the staker's position.
fn extend_lock_time(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    time: u64,
) -> Result<Response, ContractError> {
    let user = info.sender;
    let mut lock = LOCKED
        .may_load(deps.storage, user.clone())?
        .filter(|lock| !lock.amount.is_zero())
        .ok_or(ContractError::LockDoesNotExist {})?;

    // Disable the ability to extend the lock time by less than a week
    time_limits_check(time)?;

    if lock.end <= get_period(env.block.time.seconds())? {
        return Err(ContractError::LockExpired {});
    };

    // Should not exceed MAX_LOCK_TIME
    time_limits_check(EPOCH_START + lock.end * WEEK + time - env.block.time.seconds())?;
    lock.end += get_periods_count(time);
    LOCKED.save(deps.storage, user.clone(), &lock, env.block.height)?;

    checkpoint(deps, env, user, None, Some(lock.end))?;

    Ok(Response::default().add_attribute("action", "extend_lock_time"))
}

/// Expose available contract queries.
///
/// ## Queries
/// * **QueryMsg::TotalVotingPower {}** Fetch the total voting power (veSIDE supply) at the current block.
///
/// * **QueryMsg::UserVotingPower { user }** Fetch the user's voting power (veSIDE balance) at the current block.
///
/// * **QueryMsg::TotalVotingPowerAt { time }** Fetch the total voting power (veSIDE supply) at a specified timestamp.
///
/// * **QueryMsg::UserVotingPowerAt { time }** Fetch the user's voting power (veSIDE balance) at a specified timestamp.
///
/// * **QueryMsg::LockInfo { user }** Fetch a user's lock information.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::TotalVotingPower {} => to_binary(&get_total_voting_power(deps, env, None)?),
        QueryMsg::UserVotingPower { user } => {
            to_binary(&get_user_voting_power(deps, env, user, None)?)
        }
        QueryMsg::TotalVotingPowerAt { time } => {
            to_binary(&get_total_voting_power(deps, env, Some(time))?)
        }
        QueryMsg::TotalVotingPowerAtPeriod { period } => {
            to_binary(&get_total_voting_power_at_period(deps, env, period)?)
        }
        QueryMsg::UserVotingPowerAt { user, time } => {
            to_binary(&get_user_voting_power(deps, env, user, Some(time))?)
        }
        QueryMsg::UserVotingPowerAtPeriod { user, period } => {
            to_binary(&get_user_voting_power_at_period(deps, user, period)?)
        }
        QueryMsg::LockInfo { user } => to_binary(&get_user_lock_info(deps, env, user)?),
        QueryMsg::UserDepositAtHeight { user, height } => {
            to_binary(&get_user_deposit_at_height(deps, user, height)?)
        }
        QueryMsg::Config {} => {
            let config = CONFIG.load(deps.storage)?;
            to_binary(&ConfigResponse {
                admin: config.admin.to_string(),
                deposit_token: config.deposit_token.to_string(),
            })
        }
        QueryMsg::Balance { address } => to_binary(&get_user_balance(deps, env, address)?),
        QueryMsg::TokenInfo {} => to_binary(&query_token_info(deps, env)?),
    }
}

/// Return a user's lock information.
///
/// * **user** user for which we return lock information.
fn get_user_lock_info(deps: Deps, env: Env, user: String) -> StdResult<LockInfoResponse> {
    let addr = deps.api.addr_validate(&user)?;
    if let Some(lock) = LOCKED.may_load(deps.storage, addr.clone())? {
        let cur_period = get_period(env.block.time.seconds())?;
        let slope = fetch_last_checkpoint(deps.storage, &addr, cur_period)?
            .map(|(_, point)| point.slope)
            .unwrap_or_default();
        let resp = LockInfoResponse {
            amount: lock.amount,
            coefficient: calc_coefficient(lock.end - lock.last_extend_lock_period),
            start: lock.start,
            end: lock.end,
            slope,
        };
        Ok(resp)
    } else {
        Err(StdError::generic_err("User is not found"))
    }
}

/// Return a user's staked LP amount at a given block height.
///
/// * **user** user for which we return lock information.
///
/// * **block_height** block height at which we return the staked LP amount.
fn get_user_deposit_at_height(deps: Deps, user: String, block_height: u64) -> StdResult<Uint128> {
    let addr = deps.api.addr_validate(&user)?;
    let locked_opt = LOCKED.may_load_at_height(deps.storage, addr, block_height)?;
    if let Some(lock) = locked_opt {
        Ok(lock.amount)
    } else {
        Ok(Uint128::zero())
    }
}

/// Calculates a user's voting power at a given timestamp.
/// If time is None, then it calculates the user's voting power at the current block.
///
/// * **user** user/staker for which we fetch the current voting power (veSIDE balance).
///
/// * **time** timestamp at which to fetch the user's voting power (veSIDE balance).
fn get_user_voting_power(
    deps: Deps,
    env: Env,
    user: String,
    time: Option<u64>,
) -> StdResult<VotingPowerResponse> {
    let period = get_period(time.unwrap_or_else(|| env.block.time.seconds()))?;
    get_user_voting_power_at_period(deps, user, period)
}

/// Calculates a user's voting power at a given period number.
///
/// * **user** user/staker for which we fetch the current voting power (veSIDE balance).
///
/// * **period** period number at which to fetch the user's voting power (veSIDE balance).
fn get_user_voting_power_at_period(
    deps: Deps,
    user: String,
    period: u64,
) -> StdResult<VotingPowerResponse> {
    let user = deps.api.addr_validate(&user)?;
    let last_checkpoint = fetch_last_checkpoint(deps.storage, &user, period)?;

    if let Some(point) = last_checkpoint.map(|(_, point)| point) {
        // The voting power point at the specified `time` was found
        let voting_power = if point.start == period {
            point.power
        } else {
            // The point before the intended period was found, thus we can calculate the user's voting power for the period we want
            calc_voting_power(&point, period)
        };
        Ok(VotingPowerResponse { voting_power })
    } else {
        // User not found
        Ok(VotingPowerResponse {
            voting_power: Uint128::zero(),
        })
    }
}

/// Calculates a user's voting power at the current block.
///
/// * **user** user/staker for which we fetch the current voting power (veSIDE balance).
fn get_user_balance(deps: Deps, env: Env, user: String) -> StdResult<BalanceResponse> {
    let vp_response = get_user_voting_power(deps, env, user, None)?;
    Ok(BalanceResponse {
        balance: vp_response.voting_power,
    })
}

/// Calculates the total voting power (total veSIDE supply) at the given timestamp.
/// If `time` is None, then it calculates the total voting power at the current block.
///
/// * **time** timestamp at which we fetch the total voting power (veSIDE supply).
fn get_total_voting_power(
    deps: Deps,
    env: Env,
    time: Option<u64>,
) -> StdResult<VotingPowerResponse> {
    let period = get_period(time.unwrap_or_else(|| env.block.time.seconds()))?;
    get_total_voting_power_at_period(deps, env, period)
}

/// Calculates the total voting power (total veSIDE supply) at the given period number.
///
/// * **period** period number at which we fetch the total voting power (veSIDE supply).
fn get_total_voting_power_at_period(
    deps: Deps,
    env: Env,
    period: u64,
) -> StdResult<VotingPowerResponse> {
    let last_checkpoint = fetch_last_checkpoint(deps.storage, &env.contract.address, period)?;

    let point = last_checkpoint.map_or(
        Point {
            power: Uint128::zero(),
            start: period,
            end: period,
            slope: Default::default(),
        },
        |(_, point)| point,
    );

    let voting_power = if point.start == period {
        point.power
    } else {
        let scheduled_slope_changes = fetch_slope_changes(deps.storage, point.start, period)?;
        let mut init_point = point;
        for (recalc_period, scheduled_change) in scheduled_slope_changes {
            init_point = Point {
                power: calc_voting_power(&init_point, recalc_period),
                start: recalc_period,
                slope: init_point.slope - scheduled_change,
                ..init_point
            }
        }
        calc_voting_power(&init_point, period)
    };

    Ok(VotingPowerResponse { voting_power })
}

/// Fetch the veSIDE token information, such as the token name, symbol, decimals and total supply (total voting power).
fn query_token_info(deps: Deps, env: Env) -> StdResult<TokenInfoResponse> {
    let info = TOKEN_INFO.load(deps.storage)?;
    let total_vp = get_total_voting_power(deps, env, None)?;
    let res = TokenInfoResponse {
        name: info.name,
        symbol: info.symbol,
        decimals: info.decimals,
        total_supply: total_vp.voting_power,
    };
    Ok(res)
}

/// Manages contract migration.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Err(ContractError::MigrationError {})
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{Addr, Timestamp};

    use super::*;

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();
        // Instantiate an contract
        let instantiate_msg = InstantiateMsg {
            admin: "some-address".to_string(),
            guardian_addr: None,
            deposit_token: "deposit-token".to_string(),
        };
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(1700161944);
        let info = mock_info("some-address", &[]);
        let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn lock_lp_flow() {
        let mut deps = mock_dependencies();
        // Instantiate an contract
        let instantiate_msg = InstantiateMsg {
            admin: "some-address".to_string(),
            guardian_addr: None,
            deposit_token: "lp-token".to_string(),
        };
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(1700161944);
        let info = mock_info("lp-token", &[]);
        let res1 = instantiate(deps.as_mut(), env.clone(), info, instantiate_msg).unwrap();
        assert_eq!(0, res1.messages.len());

        // Minimum lock is 1 week
        let res = create_lock(
            deps.as_mut(),
            env.clone(),
            Addr::unchecked("user".to_string()),
            Uint128::from(1000u64),
            604800,
        )
        .unwrap();
        assert_eq!(0, res.messages.len());

        // query lock
        let res = get_total_voting_power(deps.as_ref(), env.clone(), None).unwrap();
        assert_eq!(res.voting_power, Uint128::from(1014u64));

        let res = get_user_balance(deps.as_ref(), env.clone(), "user".to_string()).unwrap();
        assert_eq!(res.balance, Uint128::from(1014u64));

        let res = get_user_lock_info(deps.as_ref(), env.clone(), "user".to_string()).unwrap();
        assert_eq!(res.amount, Uint128::from(1000u64));

        // Extend lock time
        let info = mock_info("user", &[]);
        let res = extend_lock_time(deps.as_mut(), env.clone(), info, 604800).unwrap();
        assert_eq!(0, res.messages.len());

        let res = get_total_voting_power(deps.as_ref(), env.clone(), None).unwrap();
        assert_eq!(res.voting_power, Uint128::from(1028u64));

        let res = get_user_balance(deps.as_ref(), env.clone(), "user".to_string()).unwrap();
        assert_eq!(res.balance, Uint128::from(1028u64));

        let res = get_user_lock_info(deps.as_ref(), env.clone(), "user".to_string()).unwrap();
        assert_eq!(res.amount, Uint128::from(1000u64));

        // Deposit more tokens
        let res = deposit_for(
            deps.as_mut(),
            env.clone(),
            Uint128::from(1000u64),
            Addr::unchecked("user".to_string()),
        )
        .unwrap();
        assert_eq!(0, res.messages.len());

        let res = get_total_voting_power(deps.as_ref(), env.clone(), None).unwrap();
        assert_eq!(res.voting_power, Uint128::from(2056u64));

        let res = get_user_balance(deps.as_ref(), env.clone(), "user".to_string()).unwrap();
        assert_eq!(res.balance, Uint128::from(2056u64));

        let res = get_user_lock_info(deps.as_ref(), env, "user".to_string()).unwrap();
        assert_eq!(res.amount, Uint128::from(2000u64));
    }

    #[test]
    fn withdraw_lock() {}
    // TODO: Add failing cases
}
