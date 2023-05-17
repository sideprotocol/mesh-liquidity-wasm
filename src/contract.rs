#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, IbcMsg, IbcTimeout, MessageInfo, Response, StdError,
    StdResult, Timestamp,
};
use sha2::{Digest, Sha256};

use cw2::set_contract_version;
use cw20::Balance;

use crate::error::ContractError;
use crate::msg::{
    AtomicSwapPacketData, CancelSwapMsg, DetailsResponse, ExecuteMsg, HeightOutput, InstantiateMsg,
    ListResponse, MakeSwapMsg, MakeSwapMsgOutput, QueryMsg, SwapMessageType, TakeSwapMsg,
};
use crate::state::{
    all_swap_order_ids,
    AtomicSwapOrder,
    Status,
    // CHANNEL_INFO,
    SWAP_ORDERS,
};
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
        ExecuteMsg::MakeSwap(msg) => execute_make_swap(deps, env, info, msg),
        ExecuteMsg::TakeSwap(msg) => execute_take_swap(deps, env, info, msg),
        ExecuteMsg::CancelSwap(msg) => execute_cancel_swap(deps, env, info, msg),
    }
}

// MakeSwap is called when the maker wants to make atomic swap. The method create new order and lock tokens.
// This is the step 1 (Create order & Lock Token) of the atomic swap: https://github.com/cosmos/ibc/tree/main/spec/app/ics-100-atomic-swap
pub fn execute_make_swap(
    _deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: MakeSwapMsg,
) -> Result<Response, ContractError> {
    // this ignores 0 value coins, must have one or more with positive balance
    let balance = Balance::from(info.funds.clone());

    if balance.is_empty() {
        return Err(ContractError::EmptyBalance {});
    }

    let ibc_packet = AtomicSwapPacketData {
        r#type: SwapMessageType::MakeSwap,
        data: to_binary(&msg)?,
        memo: String::new(),
    };

    let ibc_msg = IbcMsg::SendPacket {
        channel_id: msg.source_channel.clone(),
        data: to_binary(&ibc_packet)?,
        // timeout: msg.timeout_timestamp.into(),
        timeout: IbcTimeout::from(Timestamp::from_nanos(msg.timeout_timestamp)),
    };

    let res = Response::new()
        .add_message(ibc_msg)
        .add_attribute("action", "make_swap");
    Ok(res)
}

// TakeSwap is the step 5 (Lock Order & Lock Token) of the atomic swap: https://github.com/liangping/ibc/blob/atomic-swap/spec/app/ics-100-atomic-swap/ibcswap.png
// This method lock the order (set a value to the field "Taker") and lock Token
pub fn execute_take_swap(
    deps: DepsMut,
    _env: Env,
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

    // Make sure the maker's buy token matches the taker's sell token
    if order.maker.buy_token != msg.sell_token {
        return Err(ContractError::InvalidSellToken);
    }

    // Checks if the order has already been taken
    if order.taker != None {
        return Err(ContractError::AlreadyTakenOrder);
    }

    // If `desiredTaker` is set, only the desiredTaker can accept the order.
    if order.maker.desired_taker != "" && order.maker.desired_taker != msg.clone().taker_address {
        return Err(ContractError::InvalidTakerAddress);
    }

    // Update order state
    // Mark that the order has been occupied
    let new_order = AtomicSwapOrder {
        id: order.id.clone(),
        maker: order.maker.clone(),
        status: order.status.clone(),
        path: order.path.clone(),
        taker: Some(msg.clone()),
        cancel_timestamp: order.cancel_timestamp.clone(),
        complete_timestamp: order.complete_timestamp.clone(),
    };

    let ibc_packet = AtomicSwapPacketData {
        r#type: SwapMessageType::TakeSwap,
        data: to_binary(&msg)?,
        memo: String::new(),
    };

    let ibc_msg = IbcMsg::SendPacket {
        channel_id: extract_source_channel_for_taker_msg(&order.path)?,
        data: to_binary(&ibc_packet)?,
        timeout: IbcTimeout::from(Timestamp::from_nanos(msg.timeout_timestamp)),
    };

    SWAP_ORDERS.save(deps.storage, &order.id, &new_order)?;

    let res = Response::new()
        .add_message(ibc_msg)
        .add_attribute("action", "take_swap")
        .add_attribute("id", new_order.id.clone());
    return Ok(res);
}

// CancelSwap is the step 10 (Cancel Request) of the atomic swap: https://github.com/cosmos/ibc/tree/main/spec/app/ics-100-atomic-swap.
// It is executed on the Maker chain. Only the maker of the order can cancel the order.
pub fn execute_cancel_swap(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: CancelSwapMsg,
) -> Result<Response, ContractError> {
    let sender = info.sender.to_string();
    let order = SWAP_ORDERS.load(deps.storage, &msg.order_id);
    let order = order.unwrap();

    if sender != order.maker.maker_address {
        return Err(ContractError::InvalidSender);
    }

    // Make sure the sender is the maker of the order.
    if order.maker.maker_address != msg.maker_address {
        return Err(ContractError::InvalidMakerAddress);
    }

    // Make sure the order is in a valid state for cancellation
    if order.status != Status::Sync && order.status != Status::Initial {
        return Err(ContractError::InvalidStatus);
    }

    let packet = AtomicSwapPacketData {
        r#type: SwapMessageType::CancelSwap,
        data: to_binary(&msg)?,
        memo: String::new(),
    };

    let ibc_msg = IbcMsg::SendPacket {
        channel_id: order.maker.source_channel,
        data: to_binary(&packet)?,
        timeout: msg.timeout_timestamp.into(),
    };

    let res = Response::new()
        .add_message(ibc_msg)
        .add_attribute("action", "cancel_swap")
        .add_attribute("id", order.id.clone());
    return Ok(res);
}

pub fn generate_order_id(order_path: &str, msg: MakeSwapMsg) -> StdResult<String> {
    let prefix = order_path.as_bytes();

    let msg_output = MakeSwapMsgOutput {
        source_port: msg.source_port.clone(),
        source_channel: msg.source_channel.clone(),
        sell_token: msg.sell_token.clone(),
        buy_token: msg.buy_token.clone(),
        maker_address: msg.maker_address.clone(),
        maker_receiving_address: msg.maker_receiving_address.clone(),
        desired_taker: msg.desired_taker.clone(),
        create_timestamp: msg.create_timestamp.clone().to_string(),
        timeout_height: HeightOutput {
            revision_number: msg.timeout_height.revision_number.clone().to_string(),
            revision_height: msg.timeout_height.revision_height.clone().to_string(),
        },
        timeout_timestamp: msg.timeout_timestamp.clone().to_string(),
        expiration_timestamp: msg.expiration_timestamp.clone().to_string(),
    };

    let binding_output = to_binary(&msg_output)?;
    let msg_bytes = binding_output.as_slice();
    let res: Vec<u8> = [prefix.clone(), msg_bytes.clone()].concat();

    let hash = Sha256::digest(&res);
    let id = hex::encode(hash);

    Ok(id)
}

pub fn order_path(
    source_channel: String,
    source_port: String,
    destination_channel: String,
    destination_port: String,
    id: u64,
) -> StdResult<String> {
    let path = format!(
        "channel/{}/port/{}/channel/{}/port/{}/{}",
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
    let swap_ids = all_swap_order_ids(deps.storage, start, limit)?;

    let mut swaps = vec![];

    for i in 0..swap_ids.len() {
        let swap_order = SWAP_ORDERS.load(deps.storage, &swap_ids[i])?;
        let details = DetailsResponse {
            id: swap_ids[i].clone(),
            maker: swap_order.maker.clone(),
            status: swap_order.status.clone(),
            path: swap_order.path.clone(),
            taker: swap_order.taker.clone(),
            cancel_timestamp: swap_order.cancel_timestamp.clone(),
            complete_timestamp: swap_order.complete_timestamp.clone(),
        };
        swaps.push(details);
    }

    Ok(ListResponse { swaps })
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, from_binary};

    use crate::msg::{Height, TakeSwapMsgOutput};

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
        let balance1 = coin(100, "token1");
        let balance2 = coin(200, "token2");
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
            desired_taker: "".to_string(),
            create_timestamp: 0,
            expiration_timestamp: env.block.time.plus_seconds(100).nanos(),
            timeout_height: Height {
                revision_number: 0,
                revision_height: 0,
            },
            timeout_timestamp: env.block.time.plus_seconds(100).nanos(),
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

    #[test]
    fn test_order_id() {
        let mut deps = mock_dependencies();

        let info = mock_info("anyone", &[]);
        let env = mock_env();
        instantiate(deps.as_mut(), env.clone(), info, InstantiateMsg {}).unwrap();
        // let balance = coins(100, "tokens");
        let balance1 = coin(100, "token");
        let balance2 = coin(100, "aside");
        let source_port =
            String::from("wasm.wasm1ghd753shjuwexxywmgs4xz7x2q732vcnkm6h2pyv9s6ah3hylvrq8epk7w");
        let source_channel = String::from("channel-9");
        let destination_channel = String::from("channel-10");
        let destination_port = String::from("swap");
        let sequence = 3;

        // Cannot create, no funds
        // let info = mock_info(&sender, &[]);
        let create = MakeSwapMsg {
            source_port: source_port.clone(),
            source_channel: source_channel.clone(),
            sell_token: balance1,
            buy_token: balance2,
            maker_address: "wasm1kj2t5txvwznrdx32v6xsw46yqztsyahqwxwlve".to_string(),
            maker_receiving_address: "wasm1kj2t5txvwznrdx32v6xsw46yqztsyahqwxwlve".to_string(),
            desired_taker: "".to_string(),
            create_timestamp: 1683279635,
            expiration_timestamp: 1693399749000000000,
            timeout_height: Height {
                revision_number: 0,
                revision_height: 0,
            },
            timeout_timestamp: 1693399799000000000,
        };

        let path = order_path(
            source_channel.clone(),
            source_port.clone(),
            destination_channel.clone(),
            destination_port.clone(),
            sequence.clone(),
        )
        .unwrap();

        let order_id = generate_order_id(&path, create.clone());
        println!("order_id is {:?}", &order_id);
    }

    #[test]
    fn test_takeswap_msg_decode() {
        let mut deps = mock_dependencies();

        let info = mock_info("anyone", &[]);
        let env = mock_env();
        instantiate(deps.as_mut(), env.clone(), info, InstantiateMsg {}).unwrap();
        // let balance = coins(100, "tokens");
        let balance2 = coin(100, "aside");
        let taker_address = String::from("side1lqd386kze5355mgpncu5y52jcdhs85ckj7kdv0");
        let taker_receiving_address = String::from("wasm19zl4l2hafcdw6p99kc00znttgpdyk32a02puj2");

        let create = TakeSwapMsg {
            order_id: String::from(
                "bf4dd83fc04ea4bf565a0294ed15d189ee2d7662a1174428d3d46b46af55c7a2",
            ),
            sell_token: balance2,
            taker_address,
            taker_receiving_address,
            timeout_height: Height {
                revision_number: 0,
                revision_height: 0,
            },
            timeout_timestamp: 1693399799000000000,
            create_timestamp: 1684328527,
        };

        let create_bytes = to_binary(&create.clone()).unwrap();
        println!("create_bytes is {:?}", &create_bytes.clone().to_base64());

        let bytes = Binary::from_base64("eyJvcmRlcl9pZCI6ImJmNGRkODNmYzA0ZWE0YmY1NjVhMDI5NGVkMTVkMTg5ZWUyZDc2NjJhMTE3NDQyOGQzZDQ2YjQ2YWY1NWM3YTIiLCJzZWxsX3Rva2VuIjp7ImRlbm9tIjoiYXNpZGUiLCJhbW91bnQiOiIxMDAifSwidGFrZXJfYWRkcmVzcyI6InNpZGUxbHFkMzg2a3plNTM1NW1ncG5jdTV5NTJqY2Roczg1Y2tqN2tkdjAiLCJ0YWtlcl9yZWNlaXZpbmdfYWRkcmVzcyI6Indhc20xOXpsNGwyaGFmY2R3NnA5OWtjMDB6bnR0Z3BkeWszMmEwMnB1ajIiLCJ0aW1lb3V0X2hlaWdodCI6eyJyZXZpc2lvbl9udW1iZXIiOiIwIiwicmV2aXNpb25faGVpZ2h0IjoiOTk5OTk5NiJ9LCJ0aW1lb3V0X3RpbWVzdGFtcCI6IjE2OTMzOTk3OTkwMDAwMDAwMDAiLCJjcmVhdGVfdGltZXN0YW1wIjoiMTY4NDMyODUyNyJ9").unwrap();

        println!("bytes is {:?}", &bytes.clone());
        // let msg: TakeSwapMsg = from_binary(&bytes.clone()).unwrap();

        let msg_res: Result<TakeSwapMsg, StdError> = from_binary(&bytes);
        let msg: TakeSwapMsg;

        match msg_res {
            Ok(value) => {
                msg = value.clone();
            }
            Err(_err) => {
                let msg_output: TakeSwapMsgOutput = from_binary(&bytes).unwrap();
                msg = TakeSwapMsg {
                    order_id: msg_output.order_id.clone(),
                    sell_token: msg_output.sell_token.clone(),
                    taker_address: msg_output.taker_address.clone(),
                    taker_receiving_address: msg_output.taker_receiving_address.clone(),
                    timeout_height: Height {
                        revision_number: msg_output
                            .timeout_height
                            .revision_number
                            .clone()
                            .parse()
                            .unwrap(),
                        revision_height: msg_output
                            .timeout_height
                            .revision_height
                            .clone()
                            .parse()
                            .unwrap(),
                    },
                    timeout_timestamp: msg_output.timeout_timestamp.parse().unwrap(),
                    create_timestamp: msg_output.create_timestamp.parse().unwrap(),
                }
            }
        }
        println!("msg is {:?}", &msg);
    }
}
