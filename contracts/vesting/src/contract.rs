use std::cmp::min;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, Uint128,
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
    let config = CONFIG.load(deps.storage)?;
    let mut ok = false;
    for address in config.allowed_addresses {
        if address == info.sender {
            ok = true;
        }
    }
    if !ok {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Must be called by allowed address"
        ))));
    }

    if vesting.cliff < env.block.time.seconds() {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Cliff time must be in future"
        ))));
    }

    if vesting.vested_time % vesting.release_interval != 0 {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Remainder for vested_time / release_interval should be zero"
        ))));
    }

    // check if given tokens are received here
    let mut ok = false;
    // First token in this chain only first token needs to be verified
    for asset in info.funds {
        if asset == vesting.token {
            ok = true;
        }
    }
    if !ok {
        return Err(ContractError::Std(StdError::generic_err(
            "Funds mismatch: Funds mismatched to with message and sent values: Start vesting"
                .to_string(),
        )));
    }

    if let Some(mut val) = VESTED_TOKENS_ALL.may_load(deps.storage, info.sender.to_string())? {
        val.push(vesting);
        VESTED_TOKENS_ALL.save(deps.storage, info.sender.to_string(), &val)?;
    } else {
        VESTED_TOKENS_ALL.save(deps.storage, info.sender.to_string(), &vec![vesting])?;
    }

    let res = Response::new().add_attribute("action", "start_vesting");
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

    config.allowed_addresses = addresses;
    CONFIG.save(deps.storage, &config)?;

    let res = Response::new().add_attribute("action", "set_allowed_addresses");
    Ok(res)
}

pub fn execute_claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let vesting_details = VESTED_TOKENS_ALL.load(deps.storage, info.sender.to_string())?;
    let mut new_vesting = vec![];
    let mut send_msg;

    for mut vesting in vesting_details {
        let now = env.block.time.seconds();
        if vesting.cliff <= now {
            let mut release_count = (now - vesting.cliff) / vesting.release_interval;
            release_count += 1;
            release_count = min(
                release_count,
                vesting.vested_time / vesting.release_interval,
            );
            let release_amount = vesting.token.amount.u128()
                / (vesting.vested_time / vesting.release_interval) as u128;

            let send_amount = release_amount.checked_mul(release_count as u128).unwrap();
            let final_amount = send_amount - vesting.amount_claimed.u128();

            send_msg = BankMsg::Send {
                to_address: info.sender.into(),
                amount: vec![Coin {
                    denom: vesting.token.denom,
                    amount: Uint128::from(final_amount),
                }],
            };

            vesting.amount_claimed += Uint128::from(final_amount);
        }
        new_vesting.push(vesting);
    }

    VESTED_TOKENS_ALL.save(deps.storage, info.sender.to_string(), &new_vesting);

    let res = Response::new()
        .add_attribute("action", "claim_tokens")
        .add_message(send_msg);
    Ok(res)
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
        QueryMsg::QueryClaims { address } => to_binary(&query_claims(deps, address)?),
        QueryMsg::QueryConfig {} => to_binary(&query_config(deps)?),
        QueryMsg::QueryVestingDetails { address } => {
            to_binary(&query_vesting_details(deps, address)?)
        }
    }
}

fn query_claims(deps: Deps, address: String) -> StdResult<String> {
    let config = CONFIG.load(deps.storage)?;

    Ok(config.contract_address)
}

fn query_vesting_details(deps: Deps, address: String) -> StdResult<Vec<VestingDetails>> {
    let vesting_details = VESTED_TOKENS_ALL.load(deps.storage, address)?;
    Ok(vesting_details)
}

fn query_config(deps: Deps) -> StdResult<Config> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}
