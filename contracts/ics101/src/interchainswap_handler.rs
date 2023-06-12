use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    error::ContractError,
    msg::{AtomicSwapPacketData, CancelSwapMsg, MakeSwapMsg, SwapMessageType, TakeSwapMsg},
    state::{AtomicSwapOrder, Status, SWAP_ORDERS},
    utils::{
        decode_make_swap_msg, decode_take_swap_msg, generate_order_id, order_path, send_tokens,
    },
};
use cosmwasm_std::{
    attr, from_binary, to_binary, Addr, Binary, DepsMut, Env, IbcBasicResponse, IbcPacket,
    IbcReceiveResponse, SubMsg, Timestamp,
};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InterchainSwapPacketAcknowledgement {
    Result(Binary),
    Error(String),
}

// create a serialized success message
pub(crate) fn ack_success() -> Binary {
    let res = InterchainSwapPacketAcknowledgement::Result(b"1".into());
    to_binary(&res).unwrap()
}

// create a serialized error message
pub(crate) fn ack_fail(err: String) -> Binary {
    let res = InterchainSwapPacketAcknowledgement::Error(err);
    to_binary(&res).unwrap()
}

pub(crate) fn do_ibc_packet_receive(
    deps: DepsMut,
    env: Env,
    packet: &IbcPacket,
) -> Result<IbcReceiveResponse, ContractError> {
    let packet_data: AtomicSwapPacketData = from_binary(&packet.data)?;

    match packet_data.r#type {
        // TODO: Update these messages to interchain messages
        // Add all the functions
        // Add test for each operation
        SwapMessageType::Unspecified => {
            let res = IbcReceiveResponse::new()
                .set_ack(ack_success())
                .add_attribute("action", "receive")
                .add_attribute("success", "true");
            Ok(res)
        }
        SwapMessageType::MakeSwap => {
            let msg: MakeSwapMsg = decode_make_swap_msg(&packet_data.data.clone());
            on_received_make(deps, env, packet, msg)
        }
        SwapMessageType::TakeSwap => {
            let msg: TakeSwapMsg = decode_take_swap_msg(&packet_data.data.clone());
            on_received_take(deps, env, packet, msg)
        }
        SwapMessageType::CancelSwap => {
            let msg: CancelSwapMsg = from_binary(&packet_data.data.clone())?;
            on_received_cancel(deps, env, packet, msg)
        }
    }
}

pub(crate) fn on_received_make(
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

pub(crate) fn on_received_take(
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

pub(crate) fn on_received_cancel(
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
        cancel_timestamp: Some(Timestamp::from_seconds(
            msg.create_timestamp.parse().unwrap(),
        )),
        complete_timestamp: None,
    };

    SWAP_ORDERS.save(deps.storage, &order_id, &new_order)?;

    let res = IbcReceiveResponse::new()
        .set_ack(ack_success())
        .add_attribute("action", "receive")
        .add_attribute("success", "true");

    Ok(res)
}

// update the balance stored on this (channel, denom) index
pub(crate) fn on_packet_success(
    deps: DepsMut,
    packet: IbcPacket,
) -> Result<IbcBasicResponse, ContractError> {
    let packet_data: AtomicSwapPacketData = from_binary(&packet.data)?;

    // similar event messages like ibctransfer module
    let attributes = vec![attr("action", "acknowledge"), attr("success", "true")];

    match packet_data.r#type {
        // This is the step 4 (Acknowledge Make Packet) of the atomic swap: https://github.com/liangping/ibc/blob/atomic-swap/spec/app/ics-100-atomic-swap/ibcswap.png
        // This logic is executed when Taker chain acknowledge the make swap packet.
        SwapMessageType::Unspecified => Ok(IbcBasicResponse::new()),
        SwapMessageType::MakeSwap => {
            // let msg: MakeSwapMsg = from_binary(&packet_data.data.clone())?;
            let msg: MakeSwapMsg = decode_make_swap_msg(&packet_data.data.clone());
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
            let msg: TakeSwapMsg = decode_take_swap_msg(&packet_data.data.clone());

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
                cancel_timestamp: Some(Timestamp::from_seconds(
                    msg.create_timestamp.parse().unwrap(),
                )),
                complete_timestamp: None,
            };

            SWAP_ORDERS.save(deps.storage, &order_id, &new_order)?;
            Ok(IbcBasicResponse::new()
                .add_submessages(submsg)
                .add_attributes(attributes))
        }
    }
}

pub(crate) fn on_packet_failure(
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

pub(crate) fn refund_packet_token(
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
            // let msg: MakeSwapMsg = from_binary(&packet.data.clone())?;
            let msg: MakeSwapMsg = decode_make_swap_msg(&packet.data.clone());
            // let order_id: String = generate_order_id(packet.clone())?;
            // let swap_order: AtomicSwapOrder = SWAP_ORDERS.load(deps.storage, &order_id)?;
            let maker_address: Addr = deps.api.addr_validate(&msg.maker_address)?;
            let submsg = send_tokens(&maker_address, msg.sell_token)?;

            Ok(submsg)
        }
        // This is the step 7.2 (Unlock order and refund) of the atomic swap: https://github.com/cosmos/ibc/tree/main/spec/app/ics-100-atomic-swap
        // This step is executed on the Taker chain when Take Swap request timeout.
        SwapMessageType::TakeSwap => {
            // let msg: TakeSwapMsg = from_binary(&packet.data.clone())?;
            let msg: TakeSwapMsg = decode_take_swap_msg(&packet.data.clone());
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
