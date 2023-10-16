use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    error::ContractError,
    msg::{
        AtomicSwapPacketData, CancelBidMsg, CancelSwapMsg, Height, MakeBidMsg, MakeSwapMsg,
        SwapMessageType, TakeBidMsg, TakeSwapMsg,
    },
    state::{
        append_atomic_order, bid_key, bids, get_atomic_order, move_order_to_bottom,
        set_atomic_order, AtomicSwapOrder, Bid, BidStatus, Side, Status, ORDER_TO_COUNT,
    },
    utils::{decode_make_swap_msg, decode_take_swap_msg, maker_fee, send_tokens, taker_fee},
};
use cosmwasm_std::{
    attr, from_binary, to_binary, Addr, Binary, DepsMut, Env, IbcBasicResponse, IbcPacket,
    IbcReceiveResponse, SubMsg, Timestamp,
};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AtomicSwapPacketAcknowledgement {
    Result(Binary),
    Error(String),
}

// create a serialized success message
pub(crate) fn ack_success() -> Binary {
    let res = AtomicSwapPacketAcknowledgement::Result(b"1".into());
    to_binary(&res).unwrap()
}

// create a serialized error message
pub(crate) fn ack_fail(err: String) -> Binary {
    let res = AtomicSwapPacketAcknowledgement::Error(err);
    to_binary(&res).unwrap()
}

pub(crate) fn do_ibc_packet_receive(
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
            let msg: MakeSwapMsg = decode_make_swap_msg(&packet_data.data);
            on_received_make(deps, env, packet, msg)
        }
        SwapMessageType::TakeSwap => {
            let msg: TakeSwapMsg = decode_take_swap_msg(&packet_data.data);
            on_received_take(deps, env, packet, msg)
        }
        SwapMessageType::CancelSwap => {
            let msg: CancelSwapMsg = from_binary(&packet_data.data)?;
            on_received_cancel(deps, env, packet, msg)
        }
        SwapMessageType::MakeBid => {
            let msg: MakeBidMsg = from_binary(&packet_data.data)?;
            on_received_make_bid(deps, env, packet, msg)
        }
        SwapMessageType::TakeBid => {
            let msg: TakeBidMsg = from_binary(&packet_data.data)?;
            on_received_take_bid(deps, env, packet, msg)
        }
        SwapMessageType::CancelBid => {
            let msg: CancelBidMsg = from_binary(&packet_data.data)?;
            on_received_cancel_bid(deps, env, packet, msg)
        }
    }
}

pub(crate) fn on_received_make(
    deps: DepsMut,
    env: Env,
    packet: &IbcPacket,
    msg: MakeSwapMsg,
) -> Result<IbcReceiveResponse, ContractError> {
    let packet_data: AtomicSwapPacketData = from_binary(&packet.data)?;
    let order_id = packet_data.order_id.unwrap();
    let path = packet_data.path.unwrap();
    let swap_order = AtomicSwapOrder {
        id: order_id.clone(),
        side: Side::Remote,
        maker: msg.clone(),
        status: Status::Sync,
        taker: None,
        cancel_timestamp: None,
        complete_timestamp: None,
        path,
        create_timestamp: env.block.time.seconds(),
        min_bid_price: msg.min_bid_price,
    };

    let count_check = ORDER_TO_COUNT.may_load(deps.storage, &order_id)?;
    if let Some(_count) = count_check {
        return Err(ContractError::AlreadyExists {});
    } else {
        append_atomic_order(deps.storage, &order_id, &swap_order)?;
    }

    let res = IbcReceiveResponse::new()
        .set_ack(ack_success())
        .add_attribute("order_id", order_id)
        .add_attribute("action", "receive")
        .add_attribute("success", "true")
        .add_attribute("action", "make_swap_received");

    Ok(res)
}

pub(crate) fn on_received_take(
    deps: DepsMut,
    env: Env,
    _packet: &IbcPacket,
    msg: TakeSwapMsg,
) -> Result<IbcReceiveResponse, ContractError> {
    let order_id = msg.order_id.clone();
    let mut swap_order = get_atomic_order(deps.storage, &order_id)?;

    if msg.sell_token != swap_order.maker.buy_token {
        return Err(ContractError::InvalidSellToken);
    }

    if !swap_order.maker.desired_taker.is_empty()
        && swap_order.maker.desired_taker != msg.taker_address
    {
        return Err(ContractError::InvalidTakerAddress);
    }

    let taker_receiving_address = deps.api.addr_validate(&msg.taker_receiving_address)?;

    let (fee, taker_amount, treasury) = taker_fee(
        deps.as_ref(),
        &swap_order.maker.sell_token.amount,
        swap_order.maker.sell_token.denom.clone(),
    );
    let submsg: Vec<SubMsg> = vec![
        send_tokens(&taker_receiving_address, taker_amount)?,
        send_tokens(&treasury, fee)?,
    ];

    swap_order.status = Status::Complete;
    swap_order.taker = Some(msg.clone());
    swap_order.complete_timestamp = Some(Timestamp::from_nanos(env.block.time.nanos()));

    set_atomic_order(deps.storage, &msg.order_id, &swap_order)?;
    move_order_to_bottom(deps.storage, &msg.order_id)?;

    let res = IbcReceiveResponse::new()
        .set_ack(ack_success())
        .add_submessages(submsg)
        .add_attribute("order_id", order_id)
        .add_attribute("action", "receive")
        .add_attribute("success", "true");

    Ok(res)
}

pub(crate) fn on_received_cancel(
    deps: DepsMut,
    env: Env,
    _packet: &IbcPacket,
    msg: CancelSwapMsg,
) -> Result<IbcReceiveResponse, ContractError> {
    let order_id = msg.order_id.clone();
    let mut swap_order = get_atomic_order(deps.storage, &order_id)?;

    if swap_order.maker.maker_address != msg.maker_address {
        return Err(ContractError::InvalidMakerAddress);
    }

    if swap_order.status != Status::Sync && swap_order.status != Status::Initial {
        return Err(ContractError::InvalidStatus);
    }

    if swap_order.taker.is_some() {
        return Err(ContractError::AlreadyTakenOrder);
    }

    swap_order.status = Status::Cancel;
    swap_order.cancel_timestamp = Some(Timestamp::from_nanos(env.block.time.nanos()));
    set_atomic_order(deps.storage, &msg.order_id, &swap_order)?;

    let res = IbcReceiveResponse::new()
        .set_ack(ack_success())
        .add_attribute("order_id", order_id)
        .add_attribute("action", "receive")
        .add_attribute("success", "true");

    Ok(res)
}

pub(crate) fn on_received_make_bid(
    deps: DepsMut,
    env: Env,
    _packet: &IbcPacket,
    msg: MakeBidMsg,
) -> Result<IbcReceiveResponse, ContractError> {
    let order_id = msg.order_id.clone();
    let key = bid_key(&msg.order_id, &msg.taker_address);

    let bid: Bid = Bid {
        bid: msg.sell_token,
        order: msg.order_id,
        status: BidStatus::Placed,
        bidder: msg.taker_address,
        bidder_receiver: msg.taker_receiving_address,
        receive_timestamp: env.block.time.seconds(), //TODO get from packet
        expire_timestamp: msg.expiration_timestamp,
    };

    bids().save(deps.storage, key, &bid)?;

    let res = IbcReceiveResponse::new()
        .set_ack(ack_success())
        .add_attribute("order_id", order_id)
        .add_attribute("action", "receive")
        .add_attribute("success", "true");

    Ok(res)
}

pub(crate) fn on_received_take_bid(
    deps: DepsMut,
    env: Env,
    _packet: &IbcPacket,
    msg: TakeBidMsg,
) -> Result<IbcReceiveResponse, ContractError> {
    let order_id = msg.order_id.clone();
    let mut swap_order = get_atomic_order(deps.storage, &order_id)?;

    let key = bid_key(&msg.order_id, &msg.bidder);
    if !bids().has(deps.storage, key.clone()) {
        return Err(ContractError::BidDoesntExist);
    }

    let mut bid = bids().load(deps.storage, key.clone())?;
    bid.status = BidStatus::Executed;
    bids().save(deps.storage, key, &bid)?;

    if !swap_order.maker.desired_taker.is_empty() && swap_order.maker.desired_taker != msg.bidder {
        return Err(ContractError::InvalidTakerAddress);
    }

    let taker_receiving_address = deps.api.addr_validate(&bid.bidder_receiver)?;

    let submsg: Vec<SubMsg> = vec![send_tokens(
        &taker_receiving_address,
        swap_order.maker.sell_token.clone(),
    )?];

    let take_msg: TakeSwapMsg = TakeSwapMsg {
        order_id: order_id.clone(),
        sell_token: bid.bid,
        taker_address: bid.bidder,
        taker_receiving_address: bid.bidder_receiver,
        timeout_height: Height {
            revision_height: 1,
            revision_number: 1,
        },
        timeout_timestamp: 100,
    };
    swap_order.status = Status::Complete;
    swap_order.taker = Some(take_msg);
    swap_order.complete_timestamp = Some(Timestamp::from_nanos(env.block.time.nanos()));

    set_atomic_order(deps.storage, &msg.order_id, &swap_order)?;
    move_order_to_bottom(deps.storage, &msg.order_id)?;
    // bids().remove(deps.storage, key)?;

    let res = IbcReceiveResponse::new()
        .set_ack(ack_success())
        .add_submessages(submsg)
        .add_attribute("order_id", order_id)
        .add_attribute("action", "receive")
        .add_attribute("success", "true");

    Ok(res)
}

pub(crate) fn on_received_cancel_bid(
    deps: DepsMut,
    _env: Env,
    _packet: &IbcPacket,
    msg: CancelBidMsg,
) -> Result<IbcReceiveResponse, ContractError> {
    let order_id = msg.order_id.clone();

    let key = bid_key(&msg.order_id, &msg.bidder);
    if !bids().has(deps.storage, key.clone()) {
        return Err(ContractError::BidDoesntExist);
    }
    let mut bid = bids().load(deps.storage, key.clone())?;
    bid.status = BidStatus::Cancelled;
    bids().save(deps.storage, key, &bid)?;

    let res = IbcReceiveResponse::new()
        .set_ack(ack_success())
        .add_attribute("order_id", order_id)
        .add_attribute("action", "receive")
        .add_attribute("success", "true");

    Ok(res)
}

// update the balance stored on this (channel, denom) index
pub(crate) fn on_packet_success(
    deps: DepsMut,
    packet: IbcPacket,
    env: Env,
) -> Result<IbcBasicResponse, ContractError> {
    let packet_data: AtomicSwapPacketData = from_binary(&packet.data)?;

    // similar event messages like ibctransfer module
    let attributes = vec![attr("action", "acknowledge"), attr("success", "true")];

    match packet_data.r#type {
        // This is the step 4 (Acknowledge Make Packet) of the atomic swap: https://github.com/liangping/ibc/blob/atomic-swap/spec/app/ics-100-atomic-swap/ibcswap.png
        // This logic is executed when Taker chain acknowledge the make swap packet.
        SwapMessageType::Unspecified => Ok(IbcBasicResponse::new()),
        SwapMessageType::MakeSwap => {
            let order_id = &packet_data.order_id.unwrap();
            let mut order = get_atomic_order(deps.storage, order_id)?;
            order.status = Status::Sync;
            set_atomic_order(deps.storage, order_id, &order)?;
            Ok(IbcBasicResponse::new().add_attributes(attributes))
        }
        // This is the step 9 (Transfer Take Token & Close order): https://github.com/cosmos/ibc/tree/main/spec/app/ics-100-atomic-swap
        // The step is executed on the Taker chain.
        SwapMessageType::TakeSwap => {
            let msg: TakeSwapMsg = decode_take_swap_msg(&packet_data.data);

            let order_id = msg.order_id.clone();
            let mut swap_order = get_atomic_order(deps.storage, &order_id)?;

            let maker_receiving_address = deps
                .api
                .addr_validate(&swap_order.maker.maker_receiving_address)?;

            let (fee, maker_amount, treasury) = maker_fee(
                deps.as_ref(),
                &msg.sell_token.amount,
                msg.sell_token.denom.clone(),
            );
            let submsg: Vec<SubMsg> = vec![
                send_tokens(&maker_receiving_address, maker_amount)?,
                send_tokens(&treasury, fee)?,
            ];

            swap_order.status = Status::Complete;
            swap_order.taker = Some(msg.clone());
            swap_order.complete_timestamp = Some(Timestamp::from_nanos(env.block.time.nanos()));

            set_atomic_order(deps.storage, &order_id, &swap_order)?;
            move_order_to_bottom(deps.storage, &msg.order_id)?;

            Ok(IbcBasicResponse::new()
                .add_submessages(submsg)
                .add_attributes(attributes))
        }
        // This is the step 14 (Cancel & refund) of the atomic swap: https://github.com/cosmos/ibc/tree/main/spec/app/ics-100-atomic-swap
        // It is executed on the Maker chain.
        SwapMessageType::CancelSwap => {
            let msg: CancelSwapMsg = from_binary(&packet_data.data)?;
            let order_id = msg.order_id;
            let mut swap_order = get_atomic_order(deps.storage, &order_id)?;

            let maker_address = deps.api.addr_validate(&swap_order.maker.maker_address)?;
            let maker_msg = swap_order.maker.clone();

            let submsg = vec![send_tokens(&maker_address, maker_msg.sell_token)?];

            swap_order.status = Status::Cancel;
            swap_order.cancel_timestamp = Some(Timestamp::from_nanos(env.block.time.nanos()));

            set_atomic_order(deps.storage, &order_id, &swap_order)?;

            Ok(IbcBasicResponse::new()
                .add_submessages(submsg)
                .add_attributes(attributes))
        }
        SwapMessageType::MakeBid => {
            let msg: MakeBidMsg = from_binary(&packet_data.data)?;

            let key = bid_key(&msg.order_id, &msg.taker_address);
            let mut bid = bids().load(deps.storage, key.clone())?;

            bid.status = BidStatus::Placed;
            bids().save(deps.storage, key, &bid)?;

            Ok(IbcBasicResponse::new().add_attributes(attributes))
        }
        SwapMessageType::TakeBid => {
            let msg: TakeBidMsg = from_binary(&packet_data.data)?;
            let order_id = msg.order_id.clone();
            let mut swap_order = get_atomic_order(deps.storage, &order_id)?;

            let key = bid_key(&msg.order_id, &msg.bidder);
            if !bids().has(deps.storage, key.clone()) {
                return Err(ContractError::BidDoesntExist);
            }

            let mut bid = bids().load(deps.storage, key.clone())?;
            bid.status = BidStatus::Executed;
            bids().save(deps.storage, key, &bid)?;
            //bids().remove(deps.storage, key)?;

            let maker_receiving_address = deps
                .api
                .addr_validate(&swap_order.maker.maker_receiving_address)?;

            let submsg: Vec<SubMsg> = vec![send_tokens(&maker_receiving_address, bid.bid.clone())?];

            let take_msg: TakeSwapMsg = TakeSwapMsg {
                order_id,
                sell_token: bid.bid,
                taker_address: bid.bidder,
                taker_receiving_address: bid.bidder_receiver,
                timeout_height: Height {
                    revision_height: 1,
                    revision_number: 1,
                },
                timeout_timestamp: 100,
            };
            swap_order.status = Status::Complete;
            swap_order.taker = Some(take_msg);
            swap_order.complete_timestamp = Some(Timestamp::from_nanos(env.block.time.nanos()));

            set_atomic_order(deps.storage, &msg.order_id, &swap_order)?;
            move_order_to_bottom(deps.storage, &msg.order_id)?;
            Ok(IbcBasicResponse::new()
                .add_submessages(submsg)
                .add_attributes(attributes))
        }
        SwapMessageType::CancelBid => {
            let msg: CancelBidMsg = from_binary(&packet_data.data)?;

            let key = bid_key(&msg.order_id, &msg.bidder);
            if !bids().has(deps.storage, key.clone()) {
                return Err(ContractError::BidDoesntExist);
            }
            let mut bid = bids().load(deps.storage, key.clone())?;

            let taker_receiving_address = deps.api.addr_validate(&bid.bidder)?;
            // Refund amount
            let submsg: Vec<SubMsg> = vec![send_tokens(&taker_receiving_address, bid.bid.clone())?];

            bid.status = BidStatus::Cancelled;
            bids().save(deps.storage, key, &bid)?;
            //bids().remove(deps.storage, key)?;

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
            let msg: MakeSwapMsg = decode_make_swap_msg(&packet.data);
            let maker_address: Addr = deps.api.addr_validate(&msg.maker_address)?;
            let submsg = vec![send_tokens(&maker_address, msg.sell_token)?];
            let order_id = packet.order_id.unwrap();
            let mut order = get_atomic_order(deps.storage, &order_id)?;
            order.status = Status::Failed;
            set_atomic_order(deps.storage, &order_id, &order)?;
            Ok(submsg)
        }
        // This is the step 7.2 (Unlock order and refund) of the atomic swap: https://github.com/cosmos/ibc/tree/main/spec/app/ics-100-atomic-swap
        // This step is executed on the Taker chain when Take Swap request timeout.
        SwapMessageType::TakeSwap => {
            let msg: TakeSwapMsg = decode_take_swap_msg(&packet.data);
            let order_id: String = msg.order_id.clone();
            let mut swap_order = get_atomic_order(deps.storage, &order_id)?;
            let taker_address: Addr = deps.api.addr_validate(&msg.taker_address)?;

            let submsg = vec![send_tokens(&taker_address, msg.sell_token)?];

            swap_order.taker = None;
            swap_order.status = Status::Sync;
            set_atomic_order(deps.storage, &order_id, &swap_order)?;

            Ok(submsg)
        }
        // do nothing, only send tokens back when cancel msg is acknowledged.
        SwapMessageType::CancelSwap => Ok(vec![]),
        SwapMessageType::MakeBid => {
            let msg: MakeBidMsg = from_binary(&packet.data)?;
            let taker_address: Addr = deps.api.addr_validate(&msg.taker_address)?;
            let submsg = vec![send_tokens(&taker_address, msg.sell_token)?];
            let order_id = msg.order_id;

            // Remove bid
            let key = bid_key(&order_id, &msg.taker_address);
            let mut bid = bids().load(deps.storage, key.clone())?;
            bid.status = BidStatus::Failed;
            bids().save(deps.storage, key, &bid)?;

            Ok(submsg)
        }
        SwapMessageType::TakeBid => Ok(vec![]),
        SwapMessageType::CancelBid => Ok(vec![]),
    }
}
