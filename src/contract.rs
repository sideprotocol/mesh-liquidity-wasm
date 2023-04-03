#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, BankMsg, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, SubMsg, WasmMsg,
};
use sha2::{Digest, Sha256};

use cw2::set_contract_version;
use cw20::{Balance, Cw20Coin, Cw20CoinVerified, Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::error::ContractError;
use crate::msg::{
    is_valid_name, BalanceHuman, CreateMsg, DetailsResponse, ExecuteMsg, InstantiateMsg,
    ListResponse, QueryMsg, ReceiveMsg,
};
use crate::state::{all_swap_ids, AtomicSwap, SWAPS};
use cw_storage_plus::Bound;

// Version info, for migration info
const CONTRACT_NAME: &str = "ics100-swap";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    // No setup
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
        ExecuteMsg::Create(msg) => {
            let sent_funds = info.funds.clone();
            execute_create(deps, env, info, msg, Balance::from(sent_funds))
        }
        ExecuteMsg::Release { id, preimage } => execute_release(deps, env, id, preimage),
        ExecuteMsg::Refund { id } => execute_refund(deps, env, id),
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg),
    }
}

pub fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let msg: ReceiveMsg = from_binary(&wrapper.msg)?;
    let token = Cw20CoinVerified {
        address: info.sender,
        amount: wrapper.amount,
    };
    // we need to update the info... so the original sender is the one authorizing with these tokens
    let orig_info = MessageInfo {
        sender: deps.api.addr_validate(&wrapper.sender)?,
        funds: info.funds,
    };
    match msg {
        ReceiveMsg::Create(create) => {
            execute_create(deps, env, orig_info, create, Balance::Cw20(token))
        }
    }
}

pub fn execute_create(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: CreateMsg,
    balance: Balance,
) -> Result<Response, ContractError> {
    if !is_valid_name(&msg.id) {
        return Err(ContractError::InvalidId {});
    }

    // this ignores 0 value coins, must have one or more with positive balance
    if balance.is_empty() {
        return Err(ContractError::EmptyBalance {});
    }

    // Ensure this is 32 bytes hex-encoded, and decode
    let hash = parse_hex_32(&msg.hash)?;

    if msg.expires.is_expired(&env.block) {
        return Err(ContractError::Expired {});
    }

    let recipient = deps.api.addr_validate(&msg.recipient)?;

    let swap = AtomicSwap {
        hash: Binary(hash),
        recipient,
        source: info.sender,
        expires: msg.expires,
        balance,
    };

    // Try to store it, fail if the id already exists (unmodifiable swaps)
    SWAPS.update(deps.storage, &msg.id, |existing| match existing {
        None => Ok(swap),
        Some(_) => Err(ContractError::AlreadyExists {}),
    })?;

    let res = Response::new()
        .add_attribute("action", "create")
        .add_attribute("id", msg.id)
        .add_attribute("hash", msg.hash)
        .add_attribute("recipient", msg.recipient);
    Ok(res)
}

pub fn execute_release(
    deps: DepsMut,
    env: Env,
    id: String,
    preimage: String,
) -> Result<Response, ContractError> {
    let swap = SWAPS.load(deps.storage, &id)?;
    if swap.is_expired(&env.block) {
        return Err(ContractError::Expired {});
    }

    let hash = Sha256::digest(&parse_hex_32(&preimage)?);
    if hash.as_slice() != swap.hash.as_slice() {
        return Err(ContractError::InvalidPreimage {});
    }

    // Delete the swap
    SWAPS.remove(deps.storage, &id);

    // Send all tokens out
    let msgs = send_tokens(&swap.recipient, swap.balance)?;
    Ok(Response::new()
        .add_submessages(msgs)
        .add_attribute("action", "release")
        .add_attribute("id", id)
        .add_attribute("preimage", preimage)
        .add_attribute("to", swap.recipient.to_string()))
}

pub fn execute_refund(deps: DepsMut, env: Env, id: String) -> Result<Response, ContractError> {
    let swap = SWAPS.load(deps.storage, &id)?;
    // Anyone can try to refund, as long as the contract is expired
    if !swap.is_expired(&env.block) {
        return Err(ContractError::NotExpired {});
    }

    // We delete the swap
    SWAPS.remove(deps.storage, &id);

    let msgs = send_tokens(&swap.source, swap.balance)?;
    Ok(Response::new()
        .add_submessages(msgs)
        .add_attribute("action", "refund")
        .add_attribute("id", id)
        .add_attribute("to", swap.source.to_string()))
}

fn parse_hex_32(data: &str) -> Result<Vec<u8>, ContractError> {
    match hex::decode(data) {
        Ok(bin) => {
            if bin.len() == 32 {
                Ok(bin)
            } else {
                Err(ContractError::InvalidHash(bin.len() * 2))
            }
        }
        Err(e) => Err(ContractError::ParseError(e.to_string())),
    }
}

fn send_tokens(to: &Addr, amount: Balance) -> StdResult<Vec<SubMsg>> {
    if amount.is_empty() {
        Ok(vec![])
    } else {
        match amount {
            Balance::Native(coins) => {
                let msg = BankMsg::Send {
                    to_address: to.into(),
                    amount: coins.into_vec(),
                };
                Ok(vec![SubMsg::new(msg)])
            }
            Balance::Cw20(coin) => {
                let msg = Cw20ExecuteMsg::Transfer {
                    recipient: to.into(),
                    amount: coin.amount,
                };
                let exec = WasmMsg::Execute {
                    contract_addr: coin.address.into(),
                    msg: to_binary(&msg)?,
                    funds: vec![],
                };
                Ok(vec![SubMsg::new(exec)])
            }
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::List { start_after, limit } => to_binary(&query_list(deps, start_after, limit)?),
        QueryMsg::Details { id } => to_binary(&query_details(deps, id)?),
    }
}

fn query_details(deps: Deps, id: String) -> StdResult<DetailsResponse> {
    let swap = SWAPS.load(deps.storage, &id)?;

    // Convert balance to human balance
    let balance_human = match swap.balance {
        Balance::Native(coins) => BalanceHuman::Native(coins.into_vec()),
        Balance::Cw20(coin) => BalanceHuman::Cw20(Cw20Coin {
            address: coin.address.into(),
            amount: coin.amount,
        }),
    };

    let details = DetailsResponse {
        id,
        hash: hex::encode(swap.hash.as_slice()),
        recipient: swap.recipient.into(),
        source: swap.source.into(),
        expires: swap.expires,
        balance: balance_human,
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
    let start = start_after.map(|s| Bound::exclusive(s.as_bytes()));

    Ok(ListResponse {
        swaps: all_swap_ids(deps.storage, start, limit)?,
    })
}
