#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, IbcMsg, MessageInfo, Response, StdError, StdResult,
};
use sha2::{Digest, Sha256};

use cw2::set_contract_version;
use cw20::Balance;

use crate::error::ContractError;
use crate::msg::{
    AtomicSwapPacketData, CancelSwapMsg, DetailsResponse, ExecuteMsg, InstantiateMsg, ListResponse,
    MakeSwapMsg, QueryMsg, SwapMessageType, TakeSwapMsg,
};
use crate::state::{all_swap_order_ids, Status, SwapOrder, CHANNEL_INFO, SWAP_ORDERS};
use cw_storage_plus::Bound;

// Version info, for migration info
const CONTRACT_NAME: &str = "ics100-swap";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const TIMEOUT_DELTA: u64 = 100;

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
        ExecuteMsg::MakeSwap(msg) => execute_make_swap(deps, env, info, msg),
        ExecuteMsg::TakeSwap(msg) => execute_take_swap(deps, env, info, msg),
        ExecuteMsg::CancelSwap(msg) => execute_cancel_swap(deps, env, info, msg),
    }
}

pub fn execute_make_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: MakeSwapMsg,
) -> Result<Response, ContractError> {
    // this ignores 0 value coins, must have one or more with positive balance
    let balance = Balance::from(info.funds.clone());

    if balance.is_empty() {
        return Err(ContractError::EmptyBalance {});
    }

    let timeout = env.block.time.plus_seconds(TIMEOUT_DELTA);

    let ibc_packet = AtomicSwapPacketData {
        message_type: SwapMessageType::MakeSwap,
        data: to_binary(&msg)?,
        memo: None,
    };

    let ibc_msg = IbcMsg::SendPacket {
        channel_id: msg.source_channel.clone(),
        data: to_binary(&ibc_packet)?,
        timeout: timeout.into(),
    };

    let order_id = generate_order_id(ibc_packet.clone())?;

    let swap = SwapOrder {
        id: order_id.clone(),
        maker: msg.clone(),
        status: Status::Initial,
        taker: None,
        cancel_timestamp: None,
        complete_timestamp: None,
        path: order_path(
            msg.source_channel.clone(),
            msg.source_port.clone(),
            CHANNEL_INFO
                .load(deps.storage, &msg.source_channel)?
                .counterparty_endpoint
                .channel_id
                .clone(),
            CHANNEL_INFO
                .load(deps.storage, &msg.source_channel)?
                .counterparty_endpoint
                .port_id
                .clone(),
            order_id.clone(),
        )?,
    };

    // Try to store it, fail if the id already exists (unmodifiable swaps)
    SWAP_ORDERS.update(deps.storage, &order_id, |existing| match existing {
        None => Ok(swap),
        Some(_) => Err(ContractError::AlreadyExists {}),
    })?;

    let res = Response::new()
        .add_message(ibc_msg)
        .add_attribute("action", "make_swap")
        .add_attribute("id", order_id.clone());
    Ok(res)
}

pub fn execute_take_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: TakeSwapMsg,
) -> Result<Response, ContractError> {
    let balance = Balance::from(info.funds.clone());

    if balance.is_empty() {
        return Err(ContractError::EmptyBalance {});
    }

    let order_item = SWAP_ORDERS.load(deps.storage, &msg.order_id);
    let order = order_item.unwrap();

    if order.status != Status::Initial && order.status != Status::Sync {
        return Err(ContractError::OrderTaken);
    }

    if order.maker.buy_token != msg.sell_token {
        return Err(ContractError::InvalidSellToken);
    }

    if order.taker != None {
        return Err(ContractError::AlreadyTakenOrder);
    }

    if order.maker.desired_taker != None
        && order.maker.desired_taker != Some(msg.clone().taker_address)
    {
        return Err(ContractError::InvalidTakerAddress);
    }

    let new_order = SwapOrder {
        id: order.id.clone(),
        maker: order.maker.clone(),
        status: order.status.clone(),
        path: order.path.clone(),
        taker: Some(msg.clone()),
        cancel_timestamp: order.cancel_timestamp.clone(),
        complete_timestamp: order.complete_timestamp.clone(),
    };

    let ibc_packet = AtomicSwapPacketData {
        message_type: SwapMessageType::TakeSwap,
        data: to_binary(&msg)?,
        memo: None,
    };

    let timeout = env.block.time.plus_seconds(TIMEOUT_DELTA);

    let ibc_msg = IbcMsg::SendPacket {
        channel_id: extract_source_channel_for_taker_msg(&order.path)?,
        data: to_binary(&ibc_packet)?,
        timeout: timeout.into(),
    };

    SWAP_ORDERS.save(deps.storage, &order.id, &new_order)?;

    let res = Response::new()
        .add_message(ibc_msg)
        .add_attribute("action", "take_swap")
        .add_attribute("id", new_order.id.clone());
    return Ok(res);
}

pub fn execute_cancel_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: CancelSwapMsg,
) -> Result<Response, ContractError> {
    let sender = info.sender.to_string();
    let order = SWAP_ORDERS.load(deps.storage, &msg.order_id);
    let order = order.unwrap();

    if sender != order.maker.maker_address {
        return Err(ContractError::InvalidSender);
    }

    if order.maker.maker_address != msg.maker_address {
        return Err(ContractError::InvalidMakerAddress);
    }

    if order.status != Status::Sync && order.status != Status::Initial {
        return Err(ContractError::InvalidStatus);
    }

    let packet = AtomicSwapPacketData {
        message_type: SwapMessageType::CancelSwap,
        data: to_binary(&msg)?,
        memo: None,
    };

    let timeout = env.block.time.plus_seconds(TIMEOUT_DELTA);

    let ibc_msg = IbcMsg::SendPacket {
        channel_id: order.maker.source_channel,
        data: to_binary(&packet)?,
        timeout: timeout.into(),
    };

    let res = Response::new()
        .add_message(ibc_msg)
        .add_attribute("action", "cancel_swap")
        .add_attribute("id", order.id.clone());
    return Ok(res);
}

pub fn generate_order_id(packet: AtomicSwapPacketData) -> StdResult<String> {
    let bytes = to_binary(&packet)?;
    let hash = Sha256::digest(&bytes);
    let id = hex::encode(hash);
    Ok(id)
}

pub fn order_path(
    source_channel: String,
    source_port: String,
    destination_channel: String,
    destination_port: String,
    id: String,
) -> StdResult<String> {
    let path = format!(
        "channel/{}/port/{}/channel/{}/port/{}/sequence/{}",
        source_channel, source_port, destination_channel, destination_port, id,
    );
    Ok(path)
}

fn extract_source_channel_for_taker_msg(path: &str) -> StdResult<String> {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() < 6 {
        return Err(StdError::generic_err("Invalid path"));
    }
    Ok(parts[5].to_string())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::List { start_after, limit } => to_binary(&query_list(deps, start_after, limit)?),
        QueryMsg::Details { id } => to_binary(&query_details(deps, id)?),
    }
}

fn query_details(deps: Deps, id: String) -> StdResult<DetailsResponse> {
    let swap_order = SWAP_ORDERS.load(deps.storage, &id)?;

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
    let start = start_after.map(|s| Bound::exclusive(s.as_bytes()));

    Ok(ListResponse {
        swaps: all_swap_order_ids(deps.storage, start, limit)?,
    })
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::coins;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    use super::*;

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();

        // Instantiate an empty contract
        let instantiate_msg = InstantiateMsg {};
        let info = mock_info("anyone", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_make_swap() {
        let mut deps = mock_dependencies();

        let info = mock_info("anyone", &[]);
        let env = mock_env();
        instantiate(deps.as_mut(), env.clone(), info, InstantiateMsg {}).unwrap();

        let sender = String::from("sender0001");
        // let balance = coins(100, "tokens");
        let balance1 = Balance::from(coins(100, "token1"));
        let balance2 = Balance::from(coins(200, "token2"));
        let source_port = String::from("100");
        let source_channel = String::from("ics100-1");

        // Cannot create, no funds
        let info = mock_info(&sender, &[]);
        let create = MakeSwapMsg {
            source_port,
            source_channel,
            sell_token: balance1,
            buy_token: balance2,
            maker_address: "maker0001".to_string(),
            maker_receiving_address: "makerrcpt0001".to_string(),
            desired_taker: None,
            creation_timestamp: env.block.time,
            expiration_timestamp: env.block.time.plus_seconds(100),
            timeout_height: 0,
            timeout_timestamp: env.block.time.plus_seconds(100),
        };
        let err = execute(
            deps.as_mut(),
            env.clone(),
            info,
            ExecuteMsg::MakeSwap(create),
        )
        .unwrap_err();
        assert_eq!(err, ContractError::EmptyBalance {});
    }
}
