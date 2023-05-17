// use cw20::{Balance, Cw20ExecuteMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    contract::{generate_order_id, order_path},
    error::{ContractError, Never},
    msg::{
        AtomicSwapPacketData, CancelSwapMsg, Height, MakeSwapMsg, SwapMessageType, TakeSwapMsg,
        TakeSwapMsgOutput,
    },
    state::{AtomicSwapOrder, Status, SWAP_ORDERS},
};
use cosmwasm_std::{
    attr, entry_point, from_binary, to_binary, Addr, BankMsg, Binary, Coin, DepsMut, Env,
    IbcAcknowledgement, IbcBasicResponse, IbcChannel, IbcChannelCloseMsg, IbcChannelConnectMsg,
    IbcChannelOpenMsg, IbcOrder, IbcPacket, IbcPacketAckMsg, IbcPacketReceiveMsg,
    IbcPacketTimeoutMsg, IbcReceiveResponse, Reply, Response, StdError, StdResult, SubMsg,
    SubMsgResult, Timestamp,
};

use crate::state::{ChannelInfo, CHANNEL_INFO};

pub const ICS100_VERSION: &str = "ics100-1";
pub const ICS100_ORDERING: IbcOrder = IbcOrder::Unordered;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AtomicSwapPacketAcknowledgement {
    Result(Binary),
    Error(String),
}

// create a serialized success message
fn ack_success() -> Binary {
    let res = AtomicSwapPacketAcknowledgement::Result(b"1".into());
    to_binary(&res).unwrap()
}

// create a serialized error message
fn ack_fail(err: String) -> Binary {
    let res = AtomicSwapPacketAcknowledgement::Error(err);
    to_binary(&res).unwrap()
}

const RECEIVE_ID: u64 = 1337;
const ACK_FAILURE_ID: u64 = 0xfa17;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, reply: Reply) -> Result<Response, ContractError> {
    match reply.id {
        RECEIVE_ID => match reply.result {
            SubMsgResult::Ok(_) => Ok(Response::new()),
            SubMsgResult::Err(err) => Ok(Response::new().set_data(ack_fail(err))),
        },
        ACK_FAILURE_ID => match reply.result {
            SubMsgResult::Ok(_) => Ok(Response::new()),
            SubMsgResult::Err(err) => Ok(Response::new().set_data(ack_fail(err))),
        },
        _ => Err(ContractError::UnknownReplyId { id: reply.id }),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// enforces ordering and versioning constraints
pub fn ibc_channel_open(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelOpenMsg,
) -> Result<(), ContractError> {
    enforce_order_and_version(msg.channel(), msg.counterparty_version())?;
    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// record the channel in CHANNEL_INFO
pub fn ibc_channel_connect(
    deps: DepsMut,
    _env: Env,
    msg: IbcChannelConnectMsg,
) -> Result<IbcBasicResponse, ContractError> {
    // we need to check the counter party version in try and ack (sometimes here)
    enforce_order_and_version(msg.channel(), msg.counterparty_version())?;

    let channel: IbcChannel = msg.into();
    let info = ChannelInfo {
        id: channel.endpoint.channel_id,
        counterparty_endpoint: channel.counterparty_endpoint,
        connection_id: channel.connection_id,
    };
    CHANNEL_INFO.save(deps.storage, &info.id, &info)?;

    Ok(IbcBasicResponse::default())
}

fn enforce_order_and_version(
    channel: &IbcChannel,
    counterparty_version: Option<&str>,
) -> Result<(), ContractError> {
    if channel.version != ICS100_VERSION {
        return Err(ContractError::InvalidIbcVersion {
            version: channel.version.clone(),
        });
    }
    if let Some(version) = counterparty_version {
        if version != ICS100_VERSION {
            return Err(ContractError::InvalidIbcVersion {
                version: version.to_string(),
            });
        }
    }
    if channel.order != ICS100_ORDERING {
        return Err(ContractError::OnlyOrderedChannel {});
    }
    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_close(
    _deps: DepsMut,
    _env: Env,
    _channel: IbcChannelCloseMsg,
) -> Result<IbcBasicResponse, ContractError> {
    // TODO: what to do here?
    // we will have locked funds that need to be returned somehow
    unimplemented!();
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// Check to see if we have any balance here
/// We should not return an error if possible, but rather an acknowledgement of failure
pub fn ibc_packet_receive(
    deps: DepsMut,
    _env: Env,
    msg: IbcPacketReceiveMsg,
) -> Result<IbcReceiveResponse, Never> {
    let packet = msg.packet;

    do_ibc_packet_receive(deps, _env, &packet).or_else(|err| {
        Ok(IbcReceiveResponse::new()
            .set_ack(ack_fail(err.to_string()))
            .add_attributes(vec![
                attr("action", "receive"),
                attr("success", "false"),
                attr("error", err.to_string()),
            ]))
    })
}

fn do_ibc_packet_receive(
    deps: DepsMut,
    env: Env,
    packet: &IbcPacket,
) -> Result<IbcReceiveResponse, ContractError> {
    let packet_data: AtomicSwapPacketData = from_binary(&packet.data)?;

    match packet_data.r#type {
        SwapMessageType::Unspecified => {
            let res = IbcReceiveResponse::new()
                .set_ack(ack_success())
                .add_attribute("action", "receive")
                .add_attribute("success", "true");
            Ok(res)
        }
        SwapMessageType::MakeSwap => {
            let msg: MakeSwapMsg = from_binary(&packet_data.data.clone())?;
            on_received_make(deps, env, packet, msg)
        }
        SwapMessageType::TakeSwap => {
            let msg_res: Result<TakeSwapMsg, StdError> = from_binary(&packet_data.data.clone());
            let msg: TakeSwapMsg;

            match msg_res {
                Ok(value) => {
                    msg = value.clone();
                }
                Err(_err) => {
                    let msg_output: TakeSwapMsgOutput =
                        from_binary(&packet_data.data.clone()).unwrap();
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
            on_received_take(deps, env, packet, msg)
        }
        SwapMessageType::CancelSwap => {
            let msg: CancelSwapMsg = from_binary(&packet_data.data.clone())?;
            on_received_cancel(deps, env, packet, msg)
        }
    }
}

fn send_tokens(to: &Addr, amount: Coin) -> StdResult<Vec<SubMsg>> {
    // if amount.is_empty() {
    //     Ok(vec![])
    // } else {
    //     match amount {
    //         Balance::Native(coins) => {
    //             let msg = BankMsg::Send {
    //                 to_address: to.into(),
    //                 amount: coins.into_vec(),
    //             };
    //             Ok(vec![SubMsg::new(msg)])
    //         }
    //         Balance::Cw20(coin) => {
    //             let msg = Cw20ExecuteMsg::Transfer {
    //                 recipient: to.into(),
    //                 amount: coin.amount,
    //             };
    //             let exec = WasmMsg::Execute {
    //                 contract_addr: coin.address.into(),
    //                 msg: to_binary(&msg)?,
    //                 funds: vec![],
    //             };
    //             Ok(vec![SubMsg::new(exec)])
    //         }
    //     }
    // }
    let msg = BankMsg::Send {
        to_address: to.into(),
        amount: vec![amount],
    };
    Ok(vec![SubMsg::new(msg)])
}

fn on_received_make(
    deps: DepsMut,
    _env: Env,
    packet: &IbcPacket,
    msg: MakeSwapMsg,
) -> Result<IbcReceiveResponse, ContractError> {
    // let packet_data: AtomicSwapPacketData = from_binary(&packet.data)?;
    let path = order_path(
        msg.source_channel.clone(),
        msg.source_port.clone(),
        packet.dest.channel_id.clone(),
        packet.dest.port_id.clone(),
        packet.sequence,
    )?;
    let order_id = generate_order_id(&path, msg.clone())?;
    let swap_order = AtomicSwapOrder {
        id: order_id.clone(),
        maker: msg.clone(),
        status: Status::Initial,
        taker: None,
        cancel_timestamp: None,
        complete_timestamp: None,
        path: path.clone(),
    };

    SWAP_ORDERS.update(deps.storage, &order_id.clone(), |existing| match existing {
        None => Ok(swap_order),
        Some(_) => Err(ContractError::AlreadyExists {}),
    })?;
    let res = IbcReceiveResponse::new()
        .set_ack(ack_success())
        .add_attribute("action", "receive")
        .add_attribute("success", "true");

    Ok(res)
}

fn on_received_take(
    deps: DepsMut,
    env: Env,
    _packet: &IbcPacket,
    msg: TakeSwapMsg,
) -> Result<IbcReceiveResponse, ContractError> {
    let order_id = msg.order_id.clone();
    let swap_order = SWAP_ORDERS.load(deps.storage, &order_id)?;

    if msg.sell_token != swap_order.maker.buy_token {
        return Err(ContractError::InvalidSellToken);
    }

    if swap_order.maker.desired_taker != ""
        && swap_order.maker.desired_taker != msg.taker_address.clone()
    {
        return Err(ContractError::InvalidTakerAddress);
    }

    let taker_receiving_address = deps
        .api
        .addr_validate(&msg.taker_receiving_address.clone())?;

    let submsg: Vec<SubMsg> = send_tokens(
        &taker_receiving_address,
        swap_order.maker.sell_token.clone(),
    )?;

    let new_order = AtomicSwapOrder {
        id: order_id.clone(),
        maker: swap_order.maker.clone(),
        status: Status::Complete,
        path: swap_order.path.clone(),
        taker: Some(msg.clone()),
        cancel_timestamp: None,
        complete_timestamp: env.block.time.clone().into(),
    };
    SWAP_ORDERS.save(deps.storage, &order_id, &new_order)?;

    let res = IbcReceiveResponse::new()
        .set_ack(ack_success())
        .add_submessages(submsg)
        .add_attribute("action", "receive")
        .add_attribute("success", "true");

    Ok(res)
}

fn on_received_cancel(
    deps: DepsMut,
    _env: Env,
    _packet: &IbcPacket,
    msg: CancelSwapMsg,
) -> Result<IbcReceiveResponse, ContractError> {
    let order_id = msg.order_id;

    let swap_order = SWAP_ORDERS.load(deps.storage, &order_id)?;

    if swap_order.maker.maker_address != msg.maker_address {
        return Err(ContractError::InvalidMakerAddress);
    }

    if swap_order.status != Status::Sync && swap_order.status != Status::Initial {
        return Err(ContractError::InvalidStatus);
    }

    if swap_order.taker != None {
        return Err(ContractError::AlreadyTakenOrder);
    }

    let new_order = AtomicSwapOrder {
        id: order_id.clone(),
        maker: swap_order.maker.clone(),
        status: Status::Cancel,
        path: swap_order.path.clone(),
        taker: swap_order.taker.clone(),
        cancel_timestamp: Some(msg.creation_timestamp.clone()),
        complete_timestamp: None,
    };

    SWAP_ORDERS.save(deps.storage, &order_id, &new_order)?;

    let res = IbcReceiveResponse::new()
        .set_ack(ack_success())
        .add_attribute("action", "receive")
        .add_attribute("success", "true");

    Ok(res)
}

pub fn try_get_ack_error(ack: &IbcAcknowledgement) -> Option<String> {
    let ack: AtomicSwapPacketAcknowledgement =
	// What we can not parse is an ACK fail.
        from_binary(&ack.data).unwrap_or_else(|_| AtomicSwapPacketAcknowledgement::Error(ack.data.to_base64()));
    match ack {
        AtomicSwapPacketAcknowledgement::Error(e) => Some(e),
        _ => None,
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
// check if success or failure and update balance, or return funds
pub fn ibc_packet_ack(
    deps: DepsMut,
    _env: Env,
    msg: IbcPacketAckMsg,
) -> Result<IbcBasicResponse, ContractError> {
    if let Some(error) = try_get_ack_error(&msg.acknowledgement) {
        on_packet_failure(deps, msg.original_packet, error)
    } else {
        on_packet_success(deps, msg.original_packet)
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// return fund to original sender (same as failure in ibc_packet_ack)
pub fn ibc_packet_timeout(
    deps: DepsMut,
    _env: Env,
    msg: IbcPacketTimeoutMsg,
) -> Result<IbcBasicResponse, ContractError> {
    let packet = msg.packet;
    on_packet_failure(deps, packet, "timeout".to_string())
}

// update the balance stored on this (channel, denom) index
fn on_packet_success(deps: DepsMut, packet: IbcPacket) -> Result<IbcBasicResponse, ContractError> {
    let packet_data: AtomicSwapPacketData = from_binary(&packet.data)?;

    // similar event messages like ibctransfer module
    let attributes = vec![attr("action", "acknowledge"), attr("success", "true")];

    match packet_data.r#type {
        // This is the step 4 (Acknowledge Make Packet) of the atomic swap: https://github.com/liangping/ibc/blob/atomic-swap/spec/app/ics-100-atomic-swap/ibcswap.png
        // This logic is executed when Taker chain acknowledge the make swap packet.
        SwapMessageType::Unspecified => Ok(IbcBasicResponse::new()),
        SwapMessageType::MakeSwap => {
            let msg: MakeSwapMsg = from_binary(&packet_data.data.clone())?;
            let path = order_path(
                msg.source_channel.clone(),
                msg.source_port.clone(),
                packet.dest.channel_id.clone(),
                packet.dest.port_id.clone(),
                packet.sequence,
            )?;
            let order_id = generate_order_id(&path, msg.clone())?;
            // let swap_order = SWAP_ORDERS.load(deps.storage, &order_id)?;

            let new_order = AtomicSwapOrder {
                id: order_id.clone(),
                maker: msg.clone(),
                status: Status::Sync,
                path: path.clone(),
                taker: None,
                cancel_timestamp: None,
                complete_timestamp: None,
            };

            SWAP_ORDERS.save(deps.storage, &order_id, &new_order)?;
            Ok(IbcBasicResponse::new().add_attributes(attributes))
        }
        // This is the step 9 (Transfer Take Token & Close order): https://github.com/cosmos/ibc/tree/main/spec/app/ics-100-atomic-swap
        // The step is executed on the Taker chain.
        SwapMessageType::TakeSwap => {
            let msg_res: Result<TakeSwapMsg, StdError> = from_binary(&packet_data.data.clone());
            let msg: TakeSwapMsg;

            match msg_res {
                Ok(value) => {
                    msg = value.clone();
                }
                Err(_err) => {
                    let msg_output: TakeSwapMsgOutput =
                        from_binary(&packet_data.data.clone()).unwrap();
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

            let order_id = msg.order_id;
            let swap_order = SWAP_ORDERS.load(deps.storage, &order_id)?;

            let maker_receiving_address = deps
                .api
                .addr_validate(&swap_order.maker.maker_receiving_address)?;

            let submsg = send_tokens(&maker_receiving_address, msg.sell_token)?;

            let new_order = AtomicSwapOrder {
                id: order_id.clone(),
                maker: swap_order.maker.clone(),
                status: Status::Complete,
                path: swap_order.path.clone(),
                taker: swap_order.taker.clone(),
                cancel_timestamp: swap_order.cancel_timestamp.clone(),
                complete_timestamp: Some(Timestamp::from_seconds(msg.create_timestamp as u64)),
            };

            SWAP_ORDERS.save(deps.storage, &order_id, &new_order)?;
            Ok(IbcBasicResponse::new()
                .add_submessages(submsg)
                .add_attributes(attributes))
        }
        // This is the step 14 (Cancel & refund) of the atomic swap: https://github.com/cosmos/ibc/tree/main/spec/app/ics-100-atomic-swap
        // It is executed on the Maker chain.
        SwapMessageType::CancelSwap => {
            let msg: CancelSwapMsg = from_binary(&packet_data.data.clone())?;
            let order_id = msg.order_id;
            let swap_order = SWAP_ORDERS.load(deps.storage, &order_id)?;

            let maker_address = deps.api.addr_validate(&swap_order.maker.maker_address)?;
            let maker_msg = swap_order.maker.clone();

            let submsg = send_tokens(&maker_address, maker_msg.sell_token)?;

            let new_order = AtomicSwapOrder {
                id: order_id.clone(),
                maker: swap_order.maker.clone(),
                status: Status::Cancel,
                path: swap_order.path.clone(),
                taker: swap_order.taker.clone(),
                cancel_timestamp: Some(msg.creation_timestamp),
                complete_timestamp: None,
            };

            SWAP_ORDERS.save(deps.storage, &order_id, &new_order)?;
            Ok(IbcBasicResponse::new()
                .add_submessages(submsg)
                .add_attributes(attributes))
        }
    }
}

fn on_packet_failure(
    deps: DepsMut,
    packet: IbcPacket,
    err: String,
) -> Result<IbcBasicResponse, ContractError> {
    let packet_data: AtomicSwapPacketData = from_binary(&packet.data)?;
    let submsg = refund_packet_token(deps, packet_data)?;

    let res = IbcBasicResponse::new()
        .add_submessages(submsg)
        .add_attribute("action", "acknowledge")
        .add_attribute("success", "false")
        .add_attribute("error", err);

    Ok(res)
}

fn refund_packet_token(
    deps: DepsMut,
    packet: AtomicSwapPacketData,
) -> Result<Vec<SubMsg>, ContractError> {
    match packet.r#type {
        // This is the step 3.2 (Refund) of the atomic swap: https://github.com/liangping/ibc/blob/atomic-swap/spec/app/ics-100-atomic-swap/ibcswap.png
        // This logic will be executed when Relayer sends make swap packet to the taker chain, but the request timeout
        // and locked tokens form the first step (see the picture on the link above) MUST be returned to the account of
        // the maker on the maker chain.
        SwapMessageType::Unspecified => Ok(vec![]),
        SwapMessageType::MakeSwap => {
            let msg: MakeSwapMsg = from_binary(&packet.data.clone())?;
            // let order_id: String = generate_order_id(packet.clone())?;
            // let swap_order: AtomicSwapOrder = SWAP_ORDERS.load(deps.storage, &order_id)?;
            let maker_address: Addr = deps.api.addr_validate(&msg.maker_address)?;
            let submsg = send_tokens(&maker_address, msg.sell_token)?;

            Ok(submsg)
        }
        // This is the step 7.2 (Unlock order and refund) of the atomic swap: https://github.com/cosmos/ibc/tree/main/spec/app/ics-100-atomic-swap
        // This step is executed on the Taker chain when Take Swap request timeout.
        SwapMessageType::TakeSwap => {
            let msg: TakeSwapMsg = from_binary(&packet.data.clone())?;
            let order_id: String = msg.order_id.clone();
            let swap_order: AtomicSwapOrder = SWAP_ORDERS.load(deps.storage, &order_id)?;
            let taker_address: Addr = deps.api.addr_validate(&msg.taker_address)?;

            let submsg = send_tokens(&taker_address, msg.sell_token)?;

            let new_order: AtomicSwapOrder = AtomicSwapOrder {
                id: order_id.clone(),
                maker: swap_order.maker.clone(),
                status: Status::Initial,
                taker: None,
                cancel_timestamp: None,
                complete_timestamp: None,
                path: swap_order.path.clone(),
            };

            SWAP_ORDERS.save(deps.storage, &order_id, &new_order)?;

            Ok(submsg)
        }
        // do nothing, only send tokens back when cancel msg is acknowledged.
        SwapMessageType::CancelSwap => Ok(vec![]),
    }
}
