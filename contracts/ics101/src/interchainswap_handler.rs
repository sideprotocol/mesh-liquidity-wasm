use std::vec;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    error::ContractError,
    types::{InterchainSwapPacketData, InterchainMessageType, StateChange, MultiAssetDepositOrder, OrderStatus},
    state::{POOLS, CONFIG, MULTI_ASSET_DEPOSIT_ORDERS, POOL_TOKENS_LIST, ACTIVE_ORDERS, LOG_VOLUME},
    utils::{
        get_pool_id_with_tokens, get_coins_from_deposits, mint_tokens_cw20, send_tokens_coin, send_tokens_cw20, burn_tokens_cw20,
    }, msg::{MsgMakePoolRequest, MsgTakePoolRequest, MsgSingleAssetDepositRequest,
     MsgMultiAssetWithdrawRequest, MsgSwapRequest,
    MsgMakeMultiAssetDepositRequest, MsgTakeMultiAssetDepositRequest, MsgCancelPoolRequest, MsgCancelMultiAssetDepositRequest, LogObservation}
    ,market::{InterchainLiquidityPool, PoolStatus::{Initialized, Active, Cancelled}, InterchainMarketMaker, PoolSide},
};
use cosmwasm_std::{
    attr, from_binary, to_binary, Binary, DepsMut, Env, IbcBasicResponse, IbcPacket,
    IbcReceiveResponse, SubMsg, Coin, Uint128, StdError, Addr, from_slice, WasmMsg,
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
    let packet_data: InterchainSwapPacketData = from_slice(&packet.data)?;

    match packet_data.r#type {
        InterchainMessageType::Unspecified => {
            let res = IbcReceiveResponse::new()
                .set_ack(ack_success())
                .add_attribute("action", "receive")
                .add_attribute("success", "true");
            Ok(res)
        }
        // Save pool data
        InterchainMessageType::MakePool => {
            let msg: MsgMakePoolRequest = from_slice(&packet_data.data.clone())?;
            on_received_make_pool(deps, env, packet, msg)
        }
        InterchainMessageType::TakePool => {
            let msg: MsgTakePoolRequest = from_slice(&packet_data.data.clone())?;
            on_received_take_pool(deps, env, packet, msg)
        }
        InterchainMessageType::CancelPool => {
            let msg: MsgCancelPoolRequest = from_slice(&packet_data.data.clone())?;
            on_received_cancel_pool(deps, env, packet, msg)
        }
        InterchainMessageType::SingleAssetDeposit => {
            let msg: MsgSingleAssetDepositRequest = from_slice(&packet_data.data.clone())?;
            let state_change_data: StateChange = from_slice(&packet_data.state_change.unwrap())?;
            on_received_single_deposit(deps, env, packet, msg, state_change_data)
        }
        InterchainMessageType::MakeMultiDeposit => {
            let msg: MsgMakeMultiAssetDepositRequest = from_slice(&packet_data.data.clone())?;
            let state_change_data: StateChange = from_slice(&packet_data.state_change.unwrap())?;
            on_received_make_multi_deposit(deps, env, packet, msg, state_change_data)
        }
        InterchainMessageType::TakeMultiDeposit => {
            let msg: MsgTakeMultiAssetDepositRequest = from_slice(&packet_data.data.clone())?;
            let state_change_data: StateChange = from_slice(&packet_data.state_change.unwrap())?;
            on_received_take_multi_deposit(deps, env, packet, msg, state_change_data)
        }
        InterchainMessageType::CancelMultiDeposit => {
            let msg: MsgCancelMultiAssetDepositRequest = from_slice(&packet_data.data.clone())?;
            let state_change_data: StateChange = from_slice(&packet_data.state_change.unwrap())?;
            on_received_cancel_multi_deposit(deps, env, packet, msg, state_change_data)
        }
        InterchainMessageType::MultiWithdraw => {
            let msg: MsgMultiAssetWithdrawRequest = from_slice(&packet_data.data.clone())?;
            let state_change_data: StateChange = from_slice(&packet_data.state_change.unwrap())?;
            on_received_multi_withdraw(deps, env, packet, msg, state_change_data)
        }
        InterchainMessageType::LeftSwap => {
            let msg: MsgSwapRequest = from_binary(&packet_data.data.clone())?;
            let state_change_data: StateChange = from_slice(&packet_data.state_change.unwrap())?;
            on_received_swap(deps, env, packet, msg, state_change_data)
        }
        InterchainMessageType::RightSwap => {
            let msg: MsgSwapRequest = from_binary(&packet_data.data.clone())?;
            let state_change_data: StateChange = from_slice(&packet_data.state_change.unwrap())?;
            on_received_swap(deps, env, packet, msg, state_change_data)
        }
    }
}

pub(crate) fn on_received_make_pool(
    deps: DepsMut,
    _env: Env,
    _packet: &IbcPacket,
    msg: MsgMakePoolRequest,
) -> Result<IbcReceiveResponse, ContractError> {
    // get pool asset from tokens and weight
    if let Err(err) = msg.validate_basic() {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Failed to validate message: {}",
            err
        ))));
    }

    let mut tokens: [Coin; 2] = Default::default();
    tokens[0] = msg.liquidity[0].balance.clone();
    tokens[1] = msg.liquidity[1].balance.clone();

    let pool_id = get_pool_id_with_tokens(&tokens, msg.source_chain_id.clone(), msg.destination_chain_id.clone());

    //load pool throw error if found
   if POOLS.has(deps.storage, &pool_id.clone()) {
    return Err(ContractError::Std(StdError::generic_err(format!(
        "Pool already exists"
    ))));
   };

    let mut liquidity = vec![];
    for mut asset in msg.liquidity {
        if asset.side == PoolSide::SOURCE {
            asset.side = PoolSide::DESTINATION
        } else if asset.side == PoolSide::DESTINATION {
            asset.side = PoolSide::SOURCE
        }
        liquidity.push(asset);
    }

    let supply: Coin = Coin {amount: Uint128::from(0u64), denom: pool_id.clone()};
    let interchain_pool: InterchainLiquidityPool = InterchainLiquidityPool {
        id: pool_id.clone(),
        source_creator: msg.creator,
        destination_creator: msg.counterparty_creator,
        assets: liquidity,
        supply: supply,
        status: Initialized,
        counter_party_port: msg.source_port,
        counter_party_channel: msg.counterparty_channel,
        swap_fee: msg.swap_fee,
        source_chain_id: msg.source_chain_id,
        destination_chain_id: msg.destination_chain_id.clone(),
        pool_price: 0
    };

    POOLS.save(deps.storage, &pool_id, &interchain_pool)?;

    let res = IbcReceiveResponse::new()
        .add_attribute("pool_id", pool_id.clone())
        .add_attribute("action", "make_pool_receive")
        .add_attribute("ics101-lp-instantiate", pool_id.clone())
        .set_ack(ack_success())
        .add_attribute("action", "receive")
        .add_attribute("success", "true");

    Ok(res)
}

pub(crate) fn on_received_take_pool(
    deps: DepsMut,
    _env: Env,
    _packet: &IbcPacket,
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

    let mut tokens: [Coin; 2] = Default::default();
    tokens[0] = interchain_pool.assets[0].balance.clone();
    tokens[1] = interchain_pool.assets[1].balance.clone();

    // find number of tokens to be minted
    // Create the interchain market maker (amm).
    let amm = InterchainMarketMaker {
        //pool_id: msg.pool_id.clone(),
        pool: interchain_pool.clone(),
        fee_rate: interchain_pool.swap_fee,
    };

    let pool_tokens = amm.deposit_multi_asset(&tokens).map_err(|err| StdError::generic_err(format!("Failed to deposit multi asset: {}", err)))?;
    let mut new_shares = Uint128::from(0u128);
    for pool in pool_tokens {
        new_shares = new_shares + pool.amount;
    }
    // mint new_shares in take receive
    let sub_message;
    // Mint tokens (cw20) to the sender
    if let Some(lp_token) = POOL_TOKENS_LIST.may_load(deps.storage, &msg.pool_id.clone())? {
        sub_message = mint_tokens_cw20(msg.counter_creator, lp_token, new_shares)?;
    } else {
        // throw error token not found, initialization is done in make_pool and
        // take_pool
        return Err(ContractError::Std(StdError::generic_err(format!(
            "LP Token is not initialized"
        ))));
    }

    interchain_pool.add_supply(Coin {denom: msg.pool_id.clone(), amount: new_shares})
    .map_err(|err| StdError::generic_err(format!("Failed to add supply: {}", err)))?;
    interchain_pool.status = Active;

    POOLS.save(deps.storage, &msg.pool_id, &interchain_pool)?;

    let res = IbcReceiveResponse::new()
        .set_ack(ack_success())
        .add_submessages(sub_message).to_owned()
        .add_attribute("pool_id", msg.pool_id)
        .add_attribute("action", "take_pool_receive")
        .add_attribute("success", "true");

    Ok(res)
}

pub(crate) fn on_received_cancel_pool(
    deps: DepsMut,
    _env: Env,
    _packet: &IbcPacket,
    msg: MsgCancelPoolRequest,
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
    interchain_pool.status = Cancelled;
    POOLS.remove(deps.storage, &msg.pool_id);

    let res = IbcReceiveResponse::new()
        .set_ack(ack_success())
        .add_attribute("pool_id", msg.pool_id)
        .add_attribute("action", "cancel_pool_receive")
        .add_attribute("success", "true");

    Ok(res)
}

pub(crate) fn on_received_single_deposit(
    deps: DepsMut,
    _env: Env,
    _packet: &IbcPacket,
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
    let pool_tokens = &state_change.pool_tokens.unwrap().clone()[0];
    // increase lp token mint amount
    interchain_pool.add_asset(msg.token.clone()).map_err(|err| StdError::generic_err(format!("Failed to add asset: {}", err)))?;
    interchain_pool.add_supply(pool_tokens.clone()).map_err(|err| StdError::generic_err(format!("Failed to add supply: {}", err)))?;

    // save pool.
    POOLS.save(deps.storage, &msg.pool_id, &interchain_pool)?;

    let res = IbcReceiveResponse::new()
    .set_ack(ack_success())
    .add_attribute("pool_id", msg.pool_id)
    .add_attribute("action", "single_asset_deposit")
    .add_attribute("success", "true");

    Ok(res)
}

pub(crate) fn on_received_make_multi_deposit(
    deps: DepsMut,
    env: Env,
    _packet: &IbcPacket,
    msg: MsgMakeMultiAssetDepositRequest,
    state_change: StateChange
) -> Result<IbcReceiveResponse, ContractError> {
	// load pool throw error if found
    let interchain_pool_temp = POOLS.may_load(deps.storage, &msg.pool_id.clone())?;
    if let Some(_pool) = interchain_pool_temp {
        // Do nothing
    } else {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Pool not found"
        ))));
    }

    let mut config = CONFIG.load(deps.storage)?;
    config.counter = config.counter + 1;

    let multi_asset_order = MultiAssetDepositOrder {
        id: state_change.multi_deposit_order_id.unwrap(),
        chain_id: msg.chain_id.clone(),
        pool_id: msg.pool_id.clone(),
        source_maker: msg.deposits[0].sender.clone(),
        destination_taker: msg.deposits[1].sender.clone(),
        deposits: get_coins_from_deposits(msg.deposits.clone()),
        status: OrderStatus::Pending,
        created_at: env.block.height
    };
    let key = msg.pool_id.clone() + "-" + &multi_asset_order.id;

    MULTI_ASSET_DEPOSIT_ORDERS.save(deps.storage, key, &multi_asset_order)?;
    let ac_key = msg.deposits[0].sender.clone() + "-" + &msg.pool_id.clone() + "-" + &msg.deposits[1].sender.clone();
    ACTIVE_ORDERS.save(deps.storage, ac_key, &multi_asset_order)?;
    CONFIG.save(deps.storage, &config)?;

    let res = IbcReceiveResponse::new()
    .set_ack(ack_success())
    .add_attribute("pool_id", msg.pool_id)
    .add_attribute("action", "make_multi_asset_deposit")
    .add_attribute("success", "true");

    Ok(res)
}

pub(crate) fn on_received_take_multi_deposit(
    deps: DepsMut,
    _env: Env,
    _packet: &IbcPacket,
    msg: MsgTakeMultiAssetDepositRequest,
    state_change: StateChange
) -> Result<IbcReceiveResponse, ContractError> {
	// load pool throw error if found
    let interchain_pool_temp = POOLS.may_load(deps.storage, &msg.pool_id.clone())?;
    let mut interchain_pool;
    if let Some(pool) = interchain_pool_temp {
        interchain_pool = pool;
    } else {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Pool not found"
        ))));
    }

    // find order
    // get order
    // load orders
    let key = msg.pool_id.clone() + "-" + &msg.order_id.clone().to_string();
    let multi_asset_order_temp = MULTI_ASSET_DEPOSIT_ORDERS.may_load(deps.storage, key.clone())?;
    let mut multi_asset_order;
    if let Some(order) = multi_asset_order_temp {
        multi_asset_order = order;
        multi_asset_order.status = OrderStatus::Complete;
        let ac_key = multi_asset_order.source_maker.clone() + "-" + &msg.pool_id.clone() + "-" + &multi_asset_order.destination_taker.clone();
        ACTIVE_ORDERS.remove(deps.storage, ac_key);
    } else {
        return Err(ContractError::ErrOrderNotFound);
    }

    let pool_tokens = state_change.pool_tokens.unwrap();
    let mut new_shares = Uint128::from(0u128);
    for pool in pool_tokens {
        new_shares = new_shares + pool.amount;
    }

    let sub_message;
    // Mint tokens (cw20) to the sender
    if let Some(lp_token) = POOL_TOKENS_LIST.may_load(deps.storage, &msg.pool_id.clone())? {
        sub_message = mint_tokens_cw20(multi_asset_order.source_maker.clone(), lp_token.clone(), new_shares.clone())?;

        // Add tokens to pool supply
        interchain_pool.add_supply(Coin { denom:msg.pool_id.clone(), amount:new_shares }).map_err(|err| StdError::generic_err(format!("Failed to add supply: {}", err)))?;

        // Add assets to pool
        for asset in multi_asset_order.deposits.clone() {
            interchain_pool.add_asset(asset).map_err(|err| StdError::generic_err(format!("Failed to add asset: {}", err)))?;
        }
    } else {
        // throw error token not found, initialization is done in make_pool and
        // take_pool
        return Err(ContractError::Std(StdError::generic_err(format!(
            "LP Token is not initialized"
        ))));
    }

    MULTI_ASSET_DEPOSIT_ORDERS.save(deps.storage, key, &multi_asset_order)?;
    POOLS.save(deps.storage, &msg.pool_id.clone(), &interchain_pool)?;

    let res = IbcReceiveResponse::new()
    .set_ack(ack_success())
    .add_submessages(sub_message)
    .add_attribute("pool_id", msg.pool_id)
    .add_attribute("action", "take_multi_asset_deposit")
    .add_attribute("success", "true");

    Ok(res)
}

pub(crate) fn on_received_cancel_multi_deposit(
    deps: DepsMut,
    _env: Env,
    _packet: &IbcPacket,
    msg: MsgCancelMultiAssetDepositRequest,
    _state_change: StateChange
) -> Result<IbcReceiveResponse, ContractError> {
	// load pool throw error if found
    let interchain_pool_temp = POOLS.may_load(deps.storage, &msg.pool_id.clone())?;
    if let Some(_pool) = interchain_pool_temp {
    } else {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Pool not found"
        ))));
    }

    // find order
    // get order
    // load orders
    let key = msg.pool_id.clone() + "-" + &msg.order_id.clone().to_string();
    let multi_asset_order_temp = MULTI_ASSET_DEPOSIT_ORDERS.may_load(deps.storage, key.clone())?;
    let mut multi_asset_order;
    if let Some(order) = multi_asset_order_temp {
        multi_asset_order = order;
        multi_asset_order.status = OrderStatus::Cancelled;
        let ac_key = multi_asset_order.source_maker.clone() + "-" + &msg.pool_id.clone() + "-" + &multi_asset_order.destination_taker.clone();
        ACTIVE_ORDERS.remove(deps.storage, ac_key);
    } else {
        return Err(ContractError::ErrOrderNotFound);
    }

    MULTI_ASSET_DEPOSIT_ORDERS.save(deps.storage, key, &multi_asset_order)?;

    let res = IbcReceiveResponse::new()
    .set_ack(ack_success())
    .add_attribute("pool_id", msg.pool_id)
    .add_attribute("action", "cancel_multi_asset_deposit")
    .add_attribute("success", "true");

    Ok(res)
}

pub(crate) fn on_received_multi_withdraw(
    deps: DepsMut,
    _env: Env,
    _packet: &IbcPacket,
    msg: MsgMultiAssetWithdrawRequest,
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

    let out_assets = state_change.out_tokens.unwrap();
    let pool_tokens = state_change.pool_tokens.unwrap();
    let token = interchain_pool.find_asset_by_side(PoolSide::SOURCE)
    .map_err(|err| StdError::generic_err(format!("Failed to find asset: {}", err)))?;
    let mut sub_messages = vec![];

    // Update pool status by subtracting the supplied pool coin and output token
    for pool_asset in out_assets {
        if token.balance.denom == pool_asset.denom {
            // Unlock tokens for this chain
            sub_messages = send_tokens_coin(&Addr::unchecked(msg.counterparty_receiver.clone()), pool_asset.clone())?;
        }
        interchain_pool.subtract_asset(pool_asset.clone()).map_err(|err| StdError::generic_err(format!("Failed to subtract asset: {}", err)))?;
    }

    for pool_token in pool_tokens {
        interchain_pool.subtract_supply(pool_token).map_err(|err| StdError::generic_err(format!("Failed to subtract supply: {}", err)))?;
    }

	// Save pool
	POOLS.save(deps.storage, &msg.pool_id.clone(), &interchain_pool)?;

    let res = IbcReceiveResponse::new()
    .set_ack(ack_success())
    .add_submessages(sub_messages)
    .add_attribute("pool_id", msg.pool_id)
    .add_attribute("action", "multi_asset_withdraw")
    .add_attribute("success", "true");

    Ok(res)
}

pub(crate) fn on_received_swap(
    deps: DepsMut,
    _env: Env,
    _packet: &IbcPacket,
    msg: MsgSwapRequest,
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

    let token_out = state_change.out_tokens.unwrap();

    // send tokens
    let mut sub_messages = send_tokens_coin(&Addr::unchecked(msg.recipient), token_out.get(0).unwrap().clone())?;
    let log_token_1;
    let log_token_2;
    // Update pool status by subtracting output token and adding input token
    match msg.swap_type {
        crate::msg::SwapMsgType::LEFT => {
            interchain_pool.add_asset(msg.token_in.clone()).map_err(|err| StdError::generic_err(format!("Failed to add asset: {}", err)))?;
            interchain_pool.subtract_asset(token_out.get(0).unwrap().clone()).map_err(|err| StdError::generic_err(format!("Failed to add asset: {}", err)))?;        
            log_token_1 = msg.token_in;
            log_token_2 = token_out.get(0).unwrap().clone();
        }
        crate::msg::SwapMsgType::RIGHT => {
            // token_out here is offer amount that is needed to get msg.token_out
            interchain_pool.add_asset(token_out.get(0).unwrap().clone()).map_err(|err| StdError::generic_err(format!("Failed to add asset: {}", err)))?;
            interchain_pool.subtract_asset(msg.token_out.clone()).map_err(|err| StdError::generic_err(format!("Failed to add asset: {}", err)))?;        
            log_token_1 = msg.token_out;
            log_token_2 = token_out.get(0).unwrap().clone()
        }
    }
    
    POOLS.save(deps.storage, &msg.pool_id, &interchain_pool)?;

    // Log swap values
    let log_volume = LOG_VOLUME.may_load(deps.storage, msg.pool_id.clone())?;
    if let Some(val) = log_volume {
        let log_msg = LogObservation {
            token1: log_token_1,
            token2: log_token_2,
        };
    
        // log message
        sub_messages.push(SubMsg::new(WasmMsg::Execute {
            contract_addr: val,
            msg: to_binary(&log_msg)?,
            funds: vec![],
        }));
    }

    let res = IbcReceiveResponse::new()
    .set_ack(ack_success())
    .add_submessages(sub_messages)
    .add_attribute("pool_id", msg.pool_id)
    .add_attribute("action", "swap_asset")
    .add_attribute("success", "true");
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
    let attributes = vec![attr("success", "true")];

    match packet_data.r#type {
        // This is the step 4 (Acknowledge Make Packet) of the atomic swap: https://github.com/liangping/ibc/blob/atomic-swap/spec/app/ics-100-atomic-swap/ibcswap.png
        // This logic is executed when Taker chain acknowledge the make swap packet.
        InterchainMessageType::Unspecified => Ok(IbcBasicResponse::new()),
        InterchainMessageType::MakePool => {
            let state_change: StateChange = from_slice(&packet_data.state_change.unwrap())?;
            // pool is already saved when makePool is called.
            // mint lp tokens 
            // tokens will be minted with takePool call because then only all the assets are deposited
            Ok(IbcBasicResponse::new()
            .add_attribute("pool_id", state_change.pool_id.unwrap())
            .add_attribute("action", "make_pool_acknowledged")
            .add_attributes(attributes))
        }
        InterchainMessageType::TakePool => {
            let msg: MsgTakePoolRequest = from_binary(&packet_data.data.clone())?;
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

            let mut tokens: [Coin; 2] = Default::default();
            tokens[0] = interchain_pool.assets[0].balance.clone();
            tokens[1] = interchain_pool.assets[1].balance.clone();
        
            // find number of tokens to be minted
            // Create the interchain market maker (amm).
            let amm = InterchainMarketMaker {
                //pool_id: msg.pool_id.clone(),
                pool: interchain_pool.clone(),
                fee_rate: interchain_pool.swap_fee,
            };

            let pool_tokens = amm.deposit_multi_asset(&tokens)
            .map_err(|err| StdError::generic_err(format!("Failed to deposit multi asset: {}", err)))?;

            let mut new_shares = Uint128::from(0u128);
            for pool in pool_tokens {
                new_shares = new_shares + pool.amount;
            }

            interchain_pool.add_supply(Coin {denom: msg.pool_id.clone(), amount: new_shares})
            .map_err(|err| StdError::generic_err(format!("Failed to add supply: {}", err)))?;
            
            interchain_pool.status = Active;
            POOLS.save(deps.storage, &msg.pool_id, &interchain_pool)?;

            Ok(IbcBasicResponse::new()
            .add_attribute("pool_id", msg.pool_id)
            .add_attribute("action", "take_pool_acknowledged")
            .add_attributes(attributes))
        }
        InterchainMessageType::CancelPool => {
            let msg: MsgCancelPoolRequest = from_binary(&packet_data.data.clone())?;
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
            interchain_pool.status = Cancelled;

            // Refund tokens
            let token = interchain_pool.find_asset_by_side(PoolSide::SOURCE)
            .map_err(|err| StdError::generic_err(format!("Failed to find asset: {}", err)))?;

            send_tokens_coin(&Addr::unchecked(interchain_pool.source_creator.clone()), token.balance)?;

            POOL_TOKENS_LIST.remove(deps.storage, &msg.pool_id.clone());
            POOLS.remove(deps.storage, &msg.pool_id);

            Ok(IbcBasicResponse::new()
            .add_attribute("pool_id", msg.pool_id)
            .add_attribute("action", "cancel_pool_acknowledged")
            .add_attributes(attributes))
        }
        InterchainMessageType::SingleAssetDeposit => {
            let msg: MsgSingleAssetDepositRequest = from_binary(&packet_data.data.clone())?;
            let state_change: StateChange = from_slice(&packet_data.state_change.unwrap())?;

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

            // mint new_shares in take receive
            let sub_message;
            // Mint tokens (cw20) to the sender
            if let Some(lp_token) = POOL_TOKENS_LIST.may_load(deps.storage, &msg.pool_id.clone())? {
                sub_message = mint_tokens_cw20(msg.sender, lp_token, state_change.pool_tokens.as_ref().unwrap()[0].amount)?;
            } else {
                // throw error token not found, initialization is done in make_pool and
                // take_pool
                return Err(ContractError::Std(StdError::generic_err(format!(
                    "LP Token is not initialized"
                ))));
            }
            // update pool status
            interchain_pool.add_asset(msg.token).map_err(|err| StdError::generic_err(format!("Failed to add asset: {}", err)))?;
            interchain_pool.add_supply(state_change.pool_tokens.unwrap()[0].clone()).map_err(|err| StdError::generic_err(format!("Failed to add supply: {}", err)))?;

            POOLS.save(deps.storage, &msg.pool_id, &interchain_pool)?;

            Ok(IbcBasicResponse::new()
            .add_attribute("pool_id", msg.pool_id)
            .add_attribute("action", "single_asset_deposit_acknowledged")
            .add_attributes(attributes).add_submessages(sub_message))
        }
        InterchainMessageType::MakeMultiDeposit => {
            let msg: MsgMakeMultiAssetDepositRequest = from_binary(&packet_data.data.clone())?;
            Ok(IbcBasicResponse::new()
            .add_attribute("pool_id", msg.pool_id)
            .add_attribute("action", "make_multi_deposit_acknowledged")
            .add_attributes(attributes))
        }
        InterchainMessageType::TakeMultiDeposit => {
            let msg: MsgTakeMultiAssetDepositRequest = from_binary(&packet_data.data.clone())?;
            let state_change: StateChange = from_slice(&packet_data.state_change.unwrap())?;
            // Mint tokens in take only i.e after receiving all the assets
            // load pool throw error if found
            let interchain_pool_temp = POOLS.may_load(deps.storage, &msg.pool_id.clone())?;
            let mut interchain_pool;
            if let Some(pool) = interchain_pool_temp {
                interchain_pool = pool;
            } else {
                return Err(ContractError::Std(StdError::generic_err(format!(
                    "Pool not found"
                ))));
            }

            // find order
            // get order
            // load orders
            let key = msg.pool_id.clone() + "-" + &msg.order_id.clone().to_string();
            let multi_asset_order_temp = MULTI_ASSET_DEPOSIT_ORDERS.may_load(deps.storage, key.clone())?;
            let mut multi_asset_order;
            if let Some(order) = multi_asset_order_temp {
                multi_asset_order = order;
                multi_asset_order.status = OrderStatus::Complete;
                let ac_key = multi_asset_order.source_maker.clone() + "-" + &msg.pool_id.clone() + "-" + &multi_asset_order.destination_taker.clone();
                ACTIVE_ORDERS.remove(deps.storage, ac_key);
            } else {
                return Err(ContractError::ErrOrderNotFound);
            }

            let pool_tokens = state_change.pool_tokens.unwrap();
            let mut new_shares = Uint128::from(0u128);
            for pool in pool_tokens {
                new_shares = new_shares + pool.amount;
            }
        
            // Mint tokens (cw20) to the sender
            if let Some(_lp_token) = POOL_TOKENS_LIST.may_load(deps.storage, &msg.pool_id.clone())? {

                // Add tokens to pool supply
                interchain_pool.add_supply(Coin { denom:msg.pool_id.clone(), amount:new_shares }).map_err(|err| StdError::generic_err(format!("Failed to add supply: {}", err)))?;

                // Add assets to pool
                for asset in multi_asset_order.deposits.clone() {
                    interchain_pool.add_asset(asset).map_err(|err| StdError::generic_err(format!("Failed to add asset: {}", err)))?;
                }
            } else {
                // throw error token not found, initialization is done in make_pool and
                // take_pool
                return Err(ContractError::Std(StdError::generic_err(format!(
                    "LP Token is not initialized"
                ))));
            }

            MULTI_ASSET_DEPOSIT_ORDERS.save(deps.storage, key, &multi_asset_order)?;
            POOLS.save(deps.storage, &msg.pool_id.clone(), &interchain_pool)?;
            Ok(IbcBasicResponse::new()
            .add_attribute("pool_id", msg.pool_id)
            .add_attribute("action", "take_multi_deposit_acknowledged")
            .add_attributes(attributes))
        }
        InterchainMessageType::CancelMultiDeposit => {
            let msg: MsgCancelMultiAssetDepositRequest = from_binary(&packet_data.data.clone())?;
            // load pool throw error if found
            let interchain_pool_temp = POOLS.may_load(deps.storage, &msg.pool_id.clone())?;
            let interchain_pool;
            if let Some(pool) = interchain_pool_temp {
                interchain_pool = pool;
            } else {
                return Err(ContractError::Std(StdError::generic_err(format!(
                    "Pool not found"
                ))));
            }

            // find order
            // get order
            // load orders
            let key = msg.pool_id.clone() + "-" + &msg.order_id.clone().to_string();
            let multi_asset_order_temp = MULTI_ASSET_DEPOSIT_ORDERS.may_load(deps.storage, key.clone())?;
            let mut multi_asset_order;
            if let Some(order) = multi_asset_order_temp {
                multi_asset_order = order;
                multi_asset_order.status = OrderStatus::Cancelled;
                let ac_key = multi_asset_order.source_maker.clone() + "-" + &msg.pool_id.clone() + "-" + &multi_asset_order.destination_taker.clone();
                ACTIVE_ORDERS.remove(deps.storage, ac_key);
            } else {
                return Err(ContractError::ErrOrderNotFound);
            }

            // Refund tokens
            let token = interchain_pool.find_asset_by_side(PoolSide::SOURCE)
            .map_err(|err| StdError::generic_err(format!("Failed to find asset: {}", err)))?;

            for asset in multi_asset_order.deposits.clone() {
                if asset.denom == token.balance.denom {
                    send_tokens_coin(&Addr::unchecked(multi_asset_order.source_maker.clone()), asset)?;
                }
            }

            MULTI_ASSET_DEPOSIT_ORDERS.save(deps.storage, key, &multi_asset_order)?;
            Ok(IbcBasicResponse::new()
            .add_attribute("pool_id", msg.pool_id)
            .add_attribute("action", "cancel_multi_deposit_acknowledged")
            .add_attributes(attributes))
        }
        InterchainMessageType::MultiWithdraw => {
            // Unlock tokens for user
            let msg: MsgMultiAssetWithdrawRequest = from_binary(&packet_data.data.clone())?;
            //let state_change = packet_data.state_change.unwrap();
            let state_change: StateChange = from_slice(&packet_data.state_change.unwrap())?;

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

            let out_assets = state_change.out_tokens.unwrap();
            let pool_tokens = state_change.pool_tokens.unwrap();
            let token = interchain_pool.find_asset_by_side(PoolSide::SOURCE)
            .map_err(|err| StdError::generic_err(format!("Failed to find asset: {}", err)))?;
            let mut sub_messages = vec![];

            // Update pool status by subtracting the supplied pool coin and output token
            for pool_asset in out_assets {
                if token.balance.denom == pool_asset.denom {
                    // Unlock tokens for this chain
                    sub_messages = send_tokens_coin(&Addr::unchecked(msg.receiver.clone()), pool_asset.clone())?;
                }
                interchain_pool.subtract_asset(pool_asset.clone()).map_err(|err| StdError::generic_err(format!("Failed to subtract asset: {}", err)))?;
            }

            for pool_token in pool_tokens {
                interchain_pool.subtract_supply(pool_token).map_err(|err| StdError::generic_err(format!("Failed to subtract supply: {}", err)))?;
            }

            // Burn tokens (cw20) to the sender
            if let Some(lp_token) = POOL_TOKENS_LIST.may_load(deps.storage, &msg.pool_id.clone())? {
                sub_messages.push(burn_tokens_cw20(lp_token, msg.pool_token.amount)?);
            } else {
                // throw error token not found, initialization is done in make_pool and
                // take_pool
                return Err(ContractError::Std(StdError::generic_err(format!(
                    "LP Token is not initialized: Error"
                ))));
            }
            // Save pool
            POOLS.save(deps.storage, &msg.pool_id.clone(), &interchain_pool)?;

            Ok(IbcBasicResponse::new()
            .add_attribute("pool_id", msg.pool_id)
            .add_attribute("action", "multi_asset_withdraw_acknowledged")
            .add_attributes(attributes).add_submessages(sub_messages))
        }
        InterchainMessageType::LeftSwap => {
            let msg: MsgSwapRequest = from_binary(&packet_data.data.clone())?;
            let state_change: StateChange = from_slice(&packet_data.state_change.unwrap())?;

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

            let token_out = state_change.out_tokens.unwrap();

            // Update pool status by subtracting output token and adding input token
            interchain_pool.add_asset(msg.token_in).map_err(|err| StdError::generic_err(format!("Failed to add asset: {}", err)))?;
            interchain_pool.subtract_asset(token_out.get(0).unwrap().clone()).map_err(|err| StdError::generic_err(format!("Failed to add asset: {}", err)))?;        
        
            POOLS.save(deps.storage, &msg.pool_id, &interchain_pool)?;

            Ok(IbcBasicResponse::new()
            .add_attribute("pool_id", msg.pool_id)
            .add_attribute("action", "swap_asset_acknowledged")
            .add_attributes(attributes))
        }
        InterchainMessageType::RightSwap => {
            let msg: MsgSwapRequest = from_binary(&packet_data.data.clone())?;
            let state_change: StateChange = from_slice(&packet_data.state_change.unwrap())?;

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

            let token_out = state_change.out_tokens.unwrap();
            // Update pool status by subtracting output token and adding input token      
            // token_out here is offer amount that is needed to get msg.token_out
            interchain_pool.add_asset(token_out.get(0).unwrap().clone()).map_err(|err| StdError::generic_err(format!("Failed to add asset: {}", err)))?;
            interchain_pool.subtract_asset(msg.token_out).map_err(|err| StdError::generic_err(format!("Failed to add asset: {}", err)))?;        
        
            POOLS.save(deps.storage, &msg.pool_id, &interchain_pool)?;
            Ok(IbcBasicResponse::new()
            .add_attribute("pool_id", msg.pool_id)
            .add_attribute("action", "swap_asset_acknowledged")
            .add_attributes(attributes))
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
        InterchainMessageType::Unspecified => Ok(vec![]),
        InterchainMessageType::MakePool => {
            // remove from map and refund make tokens
            let msg: MsgMakePoolRequest = from_binary(&packet.data.clone())?;
            let mut tokens: [Coin; 2] = Default::default();
            tokens[0] = msg.liquidity[0].balance.clone();
            tokens[1] = msg.liquidity[1].balance.clone();

            let pool_id = get_pool_id_with_tokens(&tokens, msg.source_chain_id, msg.destination_chain_id);
            let sub_messages = send_tokens_coin(&Addr::unchecked(msg.creator), tokens[0].clone())?;

            POOLS.remove(deps.storage, &pool_id);
            POOL_TOKENS_LIST.remove(deps.storage, &pool_id);

            Ok(sub_messages)
        }
        InterchainMessageType::TakePool => {
            let msg: MsgTakePoolRequest = from_binary(&packet.data.clone())?;
            // load pool throw error if found
            let interchain_pool_temp = POOLS.may_load(deps.storage, &msg.pool_id)?;
            let interchain_pool;
            if let Some(pool) = interchain_pool_temp {
                interchain_pool = pool;
            } else {
                return Err(ContractError::Std(StdError::generic_err(format!(
                    "Pool not found"
                ))));
            }

            let mut tokens: [Coin; 2] = Default::default();
            tokens[0] = interchain_pool.assets[0].balance.clone();
            tokens[1] = interchain_pool.assets[1].balance.clone();

            let sub_messages = send_tokens_coin(&Addr::unchecked(msg.creator), tokens[1].clone())?;

            Ok(sub_messages)
        }
        InterchainMessageType::CancelPool => {
            // do nothing
            Ok(vec![])
        }
        InterchainMessageType::SingleAssetDeposit => {
            let msg: MsgSingleAssetDepositRequest = from_binary(&packet.data.clone())?;
            let sub_messages = send_tokens_coin(&Addr::unchecked(msg.sender), msg.token)?;

            Ok(sub_messages)
        }
        InterchainMessageType::MakeMultiDeposit => {
            let msg: MsgMakeMultiAssetDepositRequest = from_binary(&packet.data.clone())?;
            let sub_messages = send_tokens_coin(&Addr::unchecked(msg.deposits[0].clone().sender), msg.deposits.get(0).unwrap().clone().balance)?;
            let ac_key = msg.deposits[0].sender.clone() + "-" + &msg.pool_id.clone() + "-" + &msg.deposits[1].sender.clone();

            let state_change: StateChange = from_slice(&packet.state_change.unwrap())?;
            let key = msg.pool_id.clone() + &state_change.multi_deposit_order_id.unwrap();

            let mut config = CONFIG.load(deps.storage)?;
            config.counter = config.counter - 1;
            MULTI_ASSET_DEPOSIT_ORDERS.remove(deps.storage, key);

            if let Ok(Some(_active_order)) = ACTIVE_ORDERS.may_load(deps.storage, ac_key.clone()) {
                ACTIVE_ORDERS.remove(deps.storage, ac_key);
            }
            CONFIG.save(deps.storage, &config)?;
            Ok(sub_messages)
        }
        InterchainMessageType::TakeMultiDeposit => {
            let msg: MsgTakeMultiAssetDepositRequest = from_binary(&packet.data.clone())?;

            let key = msg.pool_id.clone() + "-" + &msg.order_id.clone().to_string();
            let multi_asset_order_temp = MULTI_ASSET_DEPOSIT_ORDERS.may_load(deps.storage, key.clone())?;
            let multi_asset_order;
            if let Some(order) = multi_asset_order_temp {
                multi_asset_order = order;
                // multi_asset_order.status = OrderStatus::Complete;
            } else {
                return Err(ContractError::ErrOrderNotFound);
            }

            let sub_messages = send_tokens_coin(&Addr::unchecked(msg.sender), multi_asset_order.deposits.get(1).unwrap().clone())?;

            Ok(sub_messages)
        }
        InterchainMessageType::CancelMultiDeposit => {
            // do nothing
            Ok(vec![])
        }
        InterchainMessageType::MultiWithdraw => {
            let msg: MsgMultiAssetWithdrawRequest = from_binary(&packet.data.clone())?;
            // Send tokens (cw20) to the sender
            let lp_token = POOL_TOKENS_LIST.may_load(deps.storage, &msg.pool_id.clone())?.unwrap();
            let sub_message = send_tokens_cw20(msg.receiver, lp_token, msg.pool_token.amount)?;
          
            Ok(sub_message)
        }
        InterchainMessageType::LeftSwap => {
            let msg: MsgSwapRequest = from_binary(&packet.data.clone())?;
            let sub_messages = send_tokens_coin(&Addr::unchecked(msg.sender), msg.token_in)?;

            Ok(sub_messages)
        },
        InterchainMessageType::RightSwap => {
            //let state_change = packet.state_change.unwrap();
            let state_change: StateChange = from_slice(&packet.state_change.unwrap())?;
            let msg: MsgSwapRequest = from_binary(&packet.data.clone())?;
            let sub_messages = send_tokens_coin(&Addr::unchecked(msg.sender), state_change.out_tokens.clone().unwrap().get(0).unwrap().clone())?;
            Ok(sub_messages)
        }
    }
}
