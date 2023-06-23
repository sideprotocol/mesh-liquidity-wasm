use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    error::ContractError,
    types::{InterchainSwapPacketData, InterchainMessageType, StateChange, MultiAssetDepositOrder, OrderStatus},
    state::{Status, POOLS, CONFIG, MULTI_ASSET_DEPOSIT_ORDERS, POOL_TOKENS_LIST},
    utils::{
        decode_create_pool_msg, get_pool_id_with_tokens, get_coins_from_deposits, mint_tokens_cw20,
    }, msg::{MsgMakePoolRequest, MsgTakePoolRequest, MsgSingleAssetDepositRequest,
     MsgMultiAssetWithdrawRequest, MsgSwapRequest,
    MsgMakeMultiAssetDepositRequest, MsgTakeMultiAssetDepositRequest}
    ,market::{InterchainLiquidityPool, PoolStatus::{PoolStatusInitialized, PoolStatusActive},
    InterchainMarketMaker},
};
use cosmwasm_std::{
    attr, from_binary, to_binary, Addr, Binary, DepsMut, Env, IbcBasicResponse, IbcPacket,
    IbcReceiveResponse, SubMsg, Timestamp, Coin, Uint128, StdError,
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
    let packet_data: InterchainSwapPacketData = from_binary(&packet.data)?;

    match packet_data.r#type {
        // TODO: Add test for each operation
        InterchainMessageType::Unspecified => {
            let res = IbcReceiveResponse::new()
                .set_ack(ack_success())
                .add_attribute("action", "receive")
                .add_attribute("success", "true");
            Ok(res)
        }
        // Save pool data
        InterchainMessageType::MakePool => {
            let msg: MsgMakePoolRequest = decode_create_pool_msg(&packet_data.data.clone());
            on_received_make_pool(deps, env, packet, msg)
        }
        InterchainMessageType::TakePool => {
            let msg: MsgTakePoolRequest = from_binary(&packet_data.data.clone())?;
            on_received_take_pool(deps, env, packet, msg)
        }
        InterchainMessageType::SingleAssetDeposit => {
            let msg: MsgSingleAssetDepositRequest = from_binary(&packet_data.data.clone())?;
            on_received_single_deposit(deps, env, packet, msg, packet_data.state_change.unwrap())
        }
        InterchainMessageType::MakeMultiDeposit => {
            let msg: MsgMakeMultiAssetDepositRequest = from_binary(&packet_data.data.clone())?;
            on_received_make_multi_deposit(deps, env, packet, msg, packet_data.state_change.unwrap())
        }
        InterchainMessageType::TakeMultiDeposit => {
            let msg: MsgTakeMultiAssetDepositRequest = from_binary(&packet_data.data.clone())?;
            on_received_take_multi_deposit(deps, env, packet, msg, packet_data.state_change.unwrap())
        }
        InterchainMessageType::MultiWithdraw => {
            let msg: MsgMultiAssetWithdrawRequest = from_binary(&packet_data.data.clone())?;
            on_received_multi_withdraw(deps, env, packet, msg)
        }
        InterchainMessageType::LeftSwap => {
            let msg: MsgSwapRequest = from_binary(&packet_data.data.clone())?;
            on_received_swap(deps, env, packet, msg)
        }
        InterchainMessageType::RightSwap => {
            let msg: MsgSwapRequest = from_binary(&packet_data.data.clone())?;
            on_received_swap(deps, env, packet, msg)
        }
    }
}

pub(crate) fn on_received_make_pool(
    deps: DepsMut,
    _env: Env,
    packet: &IbcPacket,
    msg: MsgMakePoolRequest,
) -> Result<IbcReceiveResponse, ContractError> {
    // get pool asset from tokens and weight
    if let Err(err) = msg.validate_basic() {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Failed to validate message: {}",
            err
        ))));
    }

    let tokens: [Coin; 2] = Default::default();
    tokens[0] = msg.liquidity[0].balance;
    tokens[1] = msg.liquidity[1].balance;

    let pool_id = get_pool_id_with_tokens(&tokens);

    // load pool throw error if found
    let interchain_pool_temp = POOLS.may_load(deps.storage, &pool_id)?;
    if let Some(pool) = interchain_pool_temp {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Pool already exists"
        ))));
    }

    let supply: Coin = Coin {amount: Uint128::from(0u64), denom: pool_id};
    let interchain_pool: InterchainLiquidityPool = InterchainLiquidityPool {
        pool_id: pool_id,
        source_creator: msg.creator,
        destination_creator: msg.counterparty_creator,
        assets: msg.liquidity,
        supply: supply,
        pool_price: 0.0,
        status: PoolStatusInitialized,
        counter_party_port: msg.source_port,
        counter_party_channel: msg.source_channel,
        swap_fee: msg.swap_fee
    };

    let amm = InterchainMarketMaker {
        pool_id: pool_id.clone(),
        pool: interchain_pool.clone(),
        fee_rate: interchain_pool.swap_fee,
    };
	interchain_pool.pool_price = amm.lp_price();

    POOLS.save(deps.storage, &pool_id, &interchain_pool)?;

    let res = IbcReceiveResponse::new()
        .set_ack(ack_success())
        .add_attribute("action", "receive")
        .add_attribute("success", "true")
        .add_attribute("sucess", "make_pool_receive");

    Ok(res)
}

pub(crate) fn on_received_take_pool(
    deps: DepsMut,
    _env: Env,
    packet: &IbcPacket,
    msg: MsgTakePoolRequest,
) -> Result<IbcReceiveResponse, ContractError> {
    // load pool throw error if found
    let interchain_pool_temp = POOLS.may_load(deps.storage, &msg.pool_id)?;
    let mut interchain_pool;
    if let Some(pool) = interchain_pool_temp {
        interchain_pool = pool;
    } else {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Pool not found"
        ))));
    }

    interchain_pool.status = PoolStatusActive;

    POOLS.save(deps.storage, &msg.pool_id, &interchain_pool)?;

    let res = IbcReceiveResponse::new()
        .set_ack(ack_success())
        .add_attribute("action", "receive")
        .add_attribute("success", "true")
        .add_attribute("sucess", "take_pool_receive");

    Ok(res)
}

pub(crate) fn on_received_single_deposit(
    deps: DepsMut,
    _env: Env,
    packet: &IbcPacket,
    msg: MsgSingleAssetDepositRequest,
    state_change: StateChange
) -> Result<IbcReceiveResponse, ContractError> {
    if let Err(err) = msg.validate_basic() {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Failed to validate message: {}",
            err
        ))));
    }

    let interchain_pool_temp = POOLS.may_load(deps.storage, &msg.pool_id)?;
    let mut interchain_pool;
    if let Some(pool) = interchain_pool_temp {
        interchain_pool = pool;
    } else {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Pool not found"
        ))));
    }
    // increase lp token mint amount
    interchain_pool.add_supply(state_change.pool_tokens.unwrap()[0]);
    // update pool tokens.
    if let Err(err) = interchain_pool.add_asset(msg.token) {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Failed to add asset: {}",
            err
        ))));
    }

    // save pool.
    POOLS.save(deps.storage, &msg.pool_id, &interchain_pool)?;

    let res = IbcReceiveResponse::new()
    .set_ack(ack_success())
    .add_attribute("action", "receive")
    .add_attribute("success", "true")
    .add_attribute("sucess", "single_asset_deposit");
    //.add_attribute("pool_token", state_change.pool_tokens);

    Ok(res)
}

pub(crate) fn on_received_make_multi_deposit(
    deps: DepsMut,
    env: Env,
    packet: &IbcPacket,
    msg: MsgMakeMultiAssetDepositRequest,
    state_change: StateChange
) -> Result<IbcReceiveResponse, ContractError> {
	// load pool throw error if found
    let interchain_pool_temp = POOLS.may_load(deps.storage, &msg.pool_id)?;
    let mut interchain_pool;
    if let Some(pool) = interchain_pool_temp {
        interchain_pool = pool;
    } else {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Pool not found"
        ))));
    }

    let mut config = CONFIG.load(deps.storage)?;
    config.counter = config.counter + 1;

    let mut multi_asset_orders: Vec<MultiAssetDepositOrder> = MULTI_ASSET_DEPOSIT_ORDERS.load(deps.storage, msg.pool_id)?;
    let mut multi_asset_order = MultiAssetDepositOrder {
        order_id: config.counter,
        pool_id: msg.pool_id,
        source_maker: msg.deposits[0].sender,
        destination_taker: msg.deposits[1].sender,
        deposits: get_coins_from_deposits(msg.deposits),
       // pool_tokens: pool_tokens,
        status: OrderStatus::Pending,
        created_at: env.block.height
    };

    multi_asset_orders.push(multi_asset_order);
    MULTI_ASSET_DEPOSIT_ORDERS.save(deps.storage, msg.pool_id, &multi_asset_orders)?;
    CONFIG.save(deps.storage, &config)?;

    let res = IbcReceiveResponse::new()
    .set_ack(ack_success())
    .add_attribute("action", "receive")
    .add_attribute("success", "true")
    .add_attribute("sucess", "make_multi_asset_deposit");
    //.add_attribute("pool_token", state_change.pool_tokens);

    Ok(res)
}

pub(crate) fn on_received_take_multi_deposit(
    deps: DepsMut,
    env: Env,
    packet: &IbcPacket,
    msg: MsgTakeMultiAssetDepositRequest,
    state_change: StateChange
) -> Result<IbcReceiveResponse, ContractError> {
	// load pool throw error if found
    let interchain_pool_temp = POOLS.may_load(deps.storage, &msg.pool_id)?;
    let mut interchain_pool;
    if let Some(pool) = interchain_pool_temp {
        interchain_pool = pool;
    } else {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Pool not found"
        ))));
    }

    // find order
    let mut multi_asset_orders: Vec<MultiAssetDepositOrder> = MULTI_ASSET_DEPOSIT_ORDERS.load(deps.storage, msg.pool_id)?;
    let mut found = false;
    let mut order;
    for  multi_order in multi_asset_orders {
        if multi_order.order_id == msg.order_id {
            found = true;
            order = multi_order
        }
    }

    if !found {
        return Err(ContractError::ErrOrderNotFound);
    }

    order.status = OrderStatus::Complete;

    let mut total_pool_tokens = Uint128::from(0u64);
    // Add tokens to pool supply
    for pool_token in state_change.pool_tokens.unwrap() {
        interchain_pool.add_supply(pool_token);
        total_pool_tokens += pool_token.amount;
    }

    // Add assets to pool
    for asset in order.deposits {
        interchain_pool.add_asset(asset);
    }

    let mut sub_message;
    // Mint tokens (cw20) to the sender
    if let Some(lp_token) = POOL_TOKENS_LIST.may_load(deps.storage, &msg.pool_id)? {
        sub_message = mint_tokens_cw20(order.source_maker, lp_token, total_pool_tokens)?;
    } else {
        // throw error token not found, initialization is done in make_pool and
        // take_pool
        return Err(ContractError::Std(StdError::generic_err(format!(
            "LP Token is not initialized"
        ))));
    }

    let mut config = CONFIG.load(deps.storage)?;
    config.counter = config.counter + 1;

    let mut multi_asset_orders: Vec<MultiAssetDepositOrder> = MULTI_ASSET_DEPOSIT_ORDERS.load(deps.storage, msg.pool_id)?;

    MULTI_ASSET_DEPOSIT_ORDERS.save(deps.storage, msg.pool_id, &multi_asset_orders)?;
    CONFIG.save(deps.storage, &config)?;
    POOLS.save(deps.storage, &msg.pool_id, &interchain_pool)?;

    let res = IbcReceiveResponse::new()
    .set_ack(ack_success())
    .add_submessages(sub_message)
    .add_attribute("action", "receive")
    .add_attribute("success", "true")
    .add_attribute("sucess", "take_multi_asset_deposit");
    //.add_attribute("pool_token", state_change.pool_tokens);

    Ok(res)
}


// update the balance stored on this (channel, denom) index
// acknowledgement
pub(crate) fn on_packet_success(
    deps: DepsMut,
    packet: IbcPacket,
) -> Result<IbcBasicResponse, ContractError> {
    let packet_data: InterchainSwapPacketData = from_binary(&packet.data)?;

    // similar event messages like ibctransfer module
    let attributes = vec![attr("action", "acknowledge"), attr("success", "true")];

    match packet_data.r#type {
        // This is the step 4 (Acknowledge Make Packet) of the atomic swap: https://github.com/liangping/ibc/blob/atomic-swap/spec/app/ics-100-atomic-swap/ibcswap.png
        // This logic is executed when Taker chain acknowledge the make swap packet.
        InterchainMessageType::Unspecified => Ok(IbcBasicResponse::new()),
        InterchainMessageType::MakePool => {
            // let msg: MakeSwapMsg = from_binary(&packet_data.data.clone())?;
            let msg: MsgMakePoolRequest = decode_create_pool_msg(&packet_data.data.clone());
            // create and save interchain pool
            // mint lp tokens 
            // calculate pool price
            Ok(IbcBasicResponse::new().add_attributes(attributes))
        }
        InterchainMessageType::TakePool => {
            // let msg: MakeSwapMsg = from_binary(&packet_data.data.clone())?;
            let msg: MsgMakePoolRequest = decode_create_pool_msg(&packet_data.data.clone());
            
            Ok(IbcBasicResponse::new().add_attributes(attributes))
        }
    }
}

pub(crate) fn on_packet_failure(
    deps: DepsMut,
    packet: IbcPacket,
    err: String,
) -> Result<IbcBasicResponse, ContractError> {
    let packet_data: InterchainSwapPacketData = from_binary(&packet.data)?;
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
    packet: InterchainSwapPacketData,
) -> Result<Vec<SubMsg>, ContractError> {
    match packet.r#type {
        // This is the step 3.2 (Refund) of the atomic swap: https://github.com/liangping/ibc/blob/atomic-swap/spec/app/ics-100-atomic-swap/ibcswap.png
        // This logic will be executed when Relayer sends make swap packet to the taker chain, but the request timeout
        // and locked tokens form the first step (see the picture on the link above) MUST be returned to the account of
        // the maker on the maker chain.
        InterchainMessageType::Unspecified => Ok(vec![]),
        InterchainMessageType::MakePool => {
            // let msg: MakeSwapMsg = from_binary(&packet.data.clone())?;
            let msg: MsgMakePoolRequest = decode_create_pool_msg(&packet.data.clone());
            // let order_id: String = generate_order_id(packet.clone())?;
            // let swap_order: AtomicSwapOrder = SWAP_ORDERS.load(deps.storage, &order_id)?;
            // let maker_address: Addr = deps.api.addr_validate(&msg.maker_address)?;
            // let submsg = send_tokens(&maker_address, msg.sell_token)?;

            Ok(submsg)
        }
        // This is the step 7.2 (Unlock order and refund) of the atomic swap: https://github.com/cosmos/ibc/tree/main/spec/app/ics-100-atomic-swap
        // This step is executed on the Taker chain when Take Swap request timeout.
        InterchainMessageType::TakePool => {
            // let msg: TakeSwapMsg = from_binary(&packet.data.clone())?;
            // let msg: TakeSwapMsg = decode_take_swap_msg(&packet.data.clone());
            // let order_id: String = msg.order_id.clone();
            // let swap_order: AtomicSwapOrder = SWAP_ORDERS.load(deps.storage, &order_id)?;
            // let taker_address: Addr = deps.api.addr_validate(&msg.taker_address)?;

            // let submsg = send_tokens(&taker_address, msg.sell_token)?;

            // let new_order: AtomicSwapOrder = AtomicSwapOrder {
            //     id: order_id.clone(),
            //     maker: swap_order.maker.clone(),
            //     status: Status::Initial,
            //     taker: None,
            //     cancel_timestamp: None,
            //     complete_timestamp: None,
            //     path: swap_order.path.clone(),
            // };

            // SWAP_ORDERS.save(deps.storage, &order_id, &new_order)?;

            Ok(submsg)
        }
        // do nothing, only send tokens back when cancel msg is acknowledged.
        InterchainMessageType::SingleAssetDeposit => Ok(vec![]),
    }
}
