use std::ops::{Div, Mul};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Coin, DepsMut, Env, IbcMsg, IbcTimeout, MessageInfo, Response, StdError, StdResult,
    Uint128, Deps, Binary, Order, SubMsg, WasmMsg, ReplyOn, Reply, from_binary, SubMsgResult,
};
use protobuf::Message;

use cw2::set_contract_version;
use cw20::{MinterResponse, Cw20ReceiveMsg, Cw20ExecuteMsg};
use cw_storage_plus::Bound;

use crate::ibc::{RECEIVE_ID, ACK_FAILURE_ID};
use crate::interchainswap_handler::ack_fail;
use crate::response::MsgInstantiateContractResponse;
use crate::error::ContractError;
use crate::market::{InterchainMarketMaker, PoolStatus, PoolSide, InterchainLiquidityPool};
use crate::msg::{
    ExecuteMsg, InstantiateMsg,
    MsgMultiAssetWithdrawRequest, MsgSingleAssetDepositRequest,
    MsgSwapRequest, SwapMsgType, MsgMakePoolRequest, MsgTakePoolRequest, MsgMakeMultiAssetDepositRequest, MsgTakeMultiAssetDepositRequest, QueryMsg, QueryConfigResponse, InterchainPoolResponse, InterchainListResponse, OrderListResponse, PoolListResponse, TokenInstantiateMsg, Cw20HookMsg, MsgCancelPoolRequest, MsgCancelMultiAssetDepositRequest, MsgRemovePool, MigrateMsg,
};
use crate::state::{POOLS, MULTI_ASSET_DEPOSIT_ORDERS, CONFIG, POOL_TOKENS_LIST, Config, TEMP, ACTIVE_ORDERS, LOG_VOLUME};
use crate::types::{InterchainSwapPacketData, StateChange, InterchainMessageType, MultiAssetDepositOrder, OrderStatus};
use crate::utils::{get_coins_from_deposits, get_pool_id_with_tokens, INSTANTIATE_TOKEN_REPLY_ID, get_order_id};

// Version info, for migration info
const CONTRACT_NAME: &str = "ics101-interchainswap";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_TIMEOUT_TIMESTAMP_OFFSET: u64 = 600;
const MAXIMUM_SLIPPAGE: u64 = 10000;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    
    let config = Config {
        counter: 0,
        token_code_id: msg.token_code_id,
        admin: info.sender.to_string(),
    };

    CONFIG.save(deps.storage, &config)?;
    Ok(Response::default())
}

/// The entry point to the contract for processing replies from submessages.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        INSTANTIATE_TOKEN_REPLY_ID => {
            let data = msg.result.clone().unwrap().data.unwrap();
            let res: MsgInstantiateContractResponse = Message::parse_from_bytes(data.as_slice())
                .map_err(|_| {
                    StdError::parse_err("MsgInstantiateContractResponse", "failed to parse data")
                })?;

            let lp_token = deps.api.addr_validate(res.get_contract_address())?;

            // Storing a temporary state using cw_storage_plus::Item and loading it into the reply handler
            // or check for events
            // Search for the instantiate event
            // let mesg = msg.result.clone().unwrap();
            // let instantiate_event = mesg.events.iter()
            // .find(|e| {
            //     e.attributes
            //         .iter()
            //         .any(|attr| attr.key == "ics101-lp-instantiate")
            // })
            // .ok_or_else(|| StdError::generic_err(format!("unable to find instantiate action")))?;

            // // Error is thrown in above line if this event is not found
            // for val in &instantiate_event.attributes {
            //     if val.key == "ics101-lp-instantiate" {
            //         POOL_TOKENS_LIST.save(deps.storage, &val.value, &lp_token.to_string())?;
            //     }
            // }

            let pool_id = TEMP.load(deps.storage).unwrap();
            TEMP.remove(deps.storage);
            POOL_TOKENS_LIST.save(deps.storage, &pool_id, &lp_token.to_string())?;
            Ok(Response::new()
                .add_attribute("liquidity_token_addr", lp_token))
        },
        RECEIVE_ID => match msg.result {
            SubMsgResult::Ok(_) => Ok(Response::new()),
            SubMsgResult::Err(err) => Ok(Response::new().set_data(ack_fail(err))),
        },
        ACK_FAILURE_ID => match msg.result {
            SubMsgResult::Ok(_) => Ok(Response::new()),
            SubMsgResult::Err(err) => Ok(Response::new().set_data(ack_fail(err))),
        },
        _ => Err(StdError::generic_err(format!("Unknown reply ID: {}", msg.id)).into()),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::MakePool(msg) => make_pool(deps, env, info, msg),
        ExecuteMsg::TakePool(msg) => take_pool(deps, env, info, msg),
        ExecuteMsg::CancelPool(msg) => cancel_pool(deps, env, info, msg),
        ExecuteMsg::SingleAssetDeposit(msg) => single_asset_deposit(deps, env, info, msg),
        ExecuteMsg::MakeMultiAssetDeposit(msg) => make_multi_asset_deposit(deps, env, info, msg),
        ExecuteMsg::CancelMultiAssetDeposit(msg) => cancel_multi_asset_deposit(deps, env, info, msg),
        ExecuteMsg::TakeMultiAssetDeposit(msg) => take_multi_asset_deposit(deps, env, info, msg),
        ExecuteMsg::MultiAssetWithdraw(msg) => multi_asset_withdraw(deps, env, info, msg),
        ExecuteMsg::Swap(msg) => swap(deps, env, info, msg),
        ExecuteMsg::RemovePool(msg) => remove_pool(deps, env, info, msg),
        ExecuteMsg::SetLogAddress { pool_id, address } => set_log_address(deps, env, info, pool_id, address)
        //ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
    }
}

fn remove_pool(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: MsgRemovePool,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if config.admin != info.sender {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "not allowed"
        ))));
    }

    POOL_TOKENS_LIST.remove(deps.storage, &msg.pool_id);
    POOLS.remove(deps.storage, &msg.pool_id);

    Ok(Response::default())
}

fn set_log_address(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    pool_id: String,
    address: String
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if config.admin != info.sender {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "not allowed"
        ))));
    }

    LOG_VOLUME.save(deps.storage, pool_id, &address)?;

    Ok(Response::default())
}

/// Receives a message of type [`Cw20ReceiveMsg`] and processes it depending on the received template.
///
/// * **cw20_msg** is the CW20 message that has to be processed.
pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::WithdrawLiquidity {
            pool_id, receiver,
            counterparty_receiver,
            timeout_height,
            timeout_timestamp }) => {
                // TODO: add sender check
                let msg: MsgMultiAssetWithdrawRequest = MsgMultiAssetWithdrawRequest {
                    pool_id: pool_id.clone(),
                    receiver: receiver,
                    counterparty_receiver: counterparty_receiver,
                    pool_token: Coin {denom: pool_id.clone(), amount: cw20_msg.amount},
                    timeout_height: timeout_height,
                    timeout_timestamp: timeout_timestamp 
                };
                multi_asset_withdraw(
                    deps,
                    env,
                    info,
                    msg
                )
            }
        Err(err) => Err(err.into()),
    }
}

fn make_pool(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: MsgMakePoolRequest,
) -> Result<Response, ContractError> {
    // validate message
    let _source_port = msg.source_port.clone();
    let source_channel = msg.source_channel.clone();

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

    TEMP.save(deps.storage, &pool_id)?;

    // load pool throw error if not found
    if POOLS.has(deps.storage,&pool_id) {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Pool already exists"
        ))));
    }

    // check if given tokens are received here
    let mut ok = false;
    // First token in this chain only first token needs to be verified
    for asset in info.funds {
        if (asset.denom == tokens[0].denom && asset.amount == tokens[0].amount) ||
            (asset.denom == tokens[1].denom && asset.amount == tokens[1].amount) {
            ok = true;
        }
    }
    if !ok {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Funds mismatch: Funds mismatched to with message and sent values: Make Pool"
        ))));
    }

    let supply: Coin = Coin {amount: Uint128::from(0u64), denom: pool_id.clone()};
    let interchain_pool: InterchainLiquidityPool = InterchainLiquidityPool {
        id: pool_id.clone(),
        source_creator: msg.creator.clone(),
        destination_creator: msg.counterparty_creator.clone(),
        assets: msg.liquidity.clone(),
        supply: supply,
        status: PoolStatus::Initialized,
        counter_party_port: msg.source_port.clone(),
        counter_party_channel: msg.source_channel.clone(),
        swap_fee: msg.swap_fee,
        source_chain_id: msg.source_chain_id.clone(),
        destination_chain_id: msg.destination_chain_id.clone(),
        pool_price: 0
    };
    POOLS.save(deps.storage, &pool_id, &interchain_pool)?;

    // Instantiate token
    let config = CONFIG.load(deps.storage)?;
    let sub_msg: Vec<SubMsg>;
    if let Some(_lp_token) = POOL_TOKENS_LIST.may_load(deps.storage, &pool_id.clone())? {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Pool token already exist: Make Pool"
        ))));
        //sub_msg = vec![];
    } else {
        // Create the LP token contract
        sub_msg = vec![SubMsg {
            msg: WasmMsg::Instantiate {
                code_id: config.token_code_id,
                msg: to_binary(&TokenInstantiateMsg {
                    name: "sideLP".to_string(),
                    symbol: "sideLP".to_string(),
                    decimals: 6,
                    initial_balances: vec![],
                    marketing: None,
                    mint: Some(MinterResponse {
                        minter: env.contract.address.to_string(),
                        cap: None,
                    }),
                })?,
                funds: vec![],
                admin: None,
                label: String::from("Sidechain LP token"),
            }
            .into(),
            id: INSTANTIATE_TOKEN_REPLY_ID,
            gas_limit: None,
            reply_on: ReplyOn::Success,
        }];
    }

    let state_change_data = to_binary(&StateChange {
        in_tokens: None,
        out_tokens: None,
        pool_tokens: None,
        pool_id: Some(pool_id.clone()),
        multi_deposit_order_id: None,
        source_chain_id: None,
    })?;

    let pool_data = to_binary(&msg)?;
    let ibc_packet_data = InterchainSwapPacketData {
        r#type: InterchainMessageType::MakePool,
        data: pool_data.clone(),
        state_change: Some(state_change_data),
    };

    let ibc_msg = IbcMsg::SendPacket {
        channel_id: source_channel.clone(),
        data: to_binary(&ibc_packet_data)?,
        timeout: IbcTimeout::from(
            env.block
                .time
                .plus_seconds(DEFAULT_TIMEOUT_TIMESTAMP_OFFSET),
        ),
    };

    let res = Response::default()
        .add_attribute("pool_id", pool_id.clone())
        .add_attribute("action", "make_pool")
        .add_attribute("ics101-lp-instantiate", pool_id.clone())
        .add_submessages(sub_msg)
        .add_message(ibc_msg);
    Ok(res)
}

fn take_pool(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: MsgTakePoolRequest,
) -> Result<Response, ContractError> {
    // load pool throw error if not found
    let interchain_pool_temp = POOLS.may_load(deps.storage, &msg.pool_id)?;
    let interchain_pool;
    if let Some(pool) = interchain_pool_temp {
        interchain_pool = pool
    } else {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Pool doesn't exist {}", msg.pool_id
        ))));
    }

    let config = CONFIG.load(deps.storage)?;
    // Send cw20 instantiate message
    let sub_msg: Vec<SubMsg>;
    if let Some(_lp_token) = POOL_TOKENS_LIST.may_load(deps.storage, &msg.pool_id.clone())? {
        // do nothing
        sub_msg = vec![];
    } else {
        // Create the LP token contract
        sub_msg = vec![SubMsg {
            msg: WasmMsg::Instantiate {
                code_id: config.token_code_id,
                msg: to_binary(&TokenInstantiateMsg {
                    name: "sideLP".to_string(),
                    symbol: "sideLP".to_string(),
                    decimals: 6,
                    initial_balances: vec![],
                    marketing: None,
                    mint: Some(MinterResponse {
                        minter: env.contract.address.to_string(),
                        cap: None,
                    }),
                })?,
                funds: vec![],
                admin: None,
                label: String::from("Sidechain LP token"),
            }
            .into(),
            id: INSTANTIATE_TOKEN_REPLY_ID,
            gas_limit: None,
            reply_on: ReplyOn::Success,
        }];
    }

    TEMP.save(deps.storage, &msg.pool_id)?;

    if interchain_pool.status != PoolStatus::Initialized {
        return Err(ContractError::InvalidStatus);
    }

    // order can only be taken by creator
    if interchain_pool.destination_creator != info.sender {
        return Err(ContractError::InvalidSender);
    }

    // check balance and funds sent handle error
    let token = interchain_pool.find_asset_by_side(PoolSide::SOURCE)
    .map_err(|err| StdError::generic_err(format!("Failed to find asset: {}", err)))?;
    // check if given tokens are received here
    let mut ok = false;
    for asset in info.funds {
        if asset.denom == token.balance.denom && asset.amount == token.balance.amount {
            ok = true;
        }
    }
    if !ok {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Funds mismatch: Funds mismatched to with message and sent values: Take Pool"
        ))));
    }

    let pool_data = to_binary(&msg).unwrap();
    let ibc_packet_data = InterchainSwapPacketData {
        r#type: InterchainMessageType::TakePool,
        data: pool_data.clone(),
        state_change: None,
    };

    // TODO: if that relayer is died, so can't recover that port and channel so have to use new relayer?  
    let ibc_msg = IbcMsg::SendPacket {
        channel_id: interchain_pool.counter_party_channel.clone(),
        data: to_binary(&ibc_packet_data)?,
        timeout: IbcTimeout::from(
            env.block
                .time
                .plus_seconds(DEFAULT_TIMEOUT_TIMESTAMP_OFFSET),
        ),
    };

    let res = Response::default()
        .add_submessages(sub_msg)
        .add_message(ibc_msg)
        .add_attribute("pool_id", msg.pool_id.clone())
        .add_attribute("action", "take_pool");
    Ok(res)
}

fn cancel_pool(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: MsgCancelPoolRequest,
) -> Result<Response, ContractError> {
    // load pool throw error if not found
    let config = CONFIG.load(deps.storage)?;
    let interchain_pool_temp = POOLS.may_load(deps.storage, &msg.pool_id)?;
    let interchain_pool;
    if let Some(pool) = interchain_pool_temp {
        interchain_pool = pool
    } else {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Pool doesn't exist {}", msg.pool_id
        ))));
    }

    if interchain_pool.status != PoolStatus::Initialized {
        return Err(ContractError::InvalidStatus);
    }

    // order can only be cancelled by creator or admin
    if !((interchain_pool.source_creator == info.sender) || (info.sender == config.admin)) {
        return Err(ContractError::InvalidSender);
    }
    
    let pool_data = to_binary(&msg).unwrap();
    let ibc_packet_data = InterchainSwapPacketData {
        r#type: InterchainMessageType::CancelPool,
        data: pool_data.clone(),
        state_change: None,
    };

    let ibc_msg = IbcMsg::SendPacket {
        channel_id: interchain_pool.counter_party_channel.clone(),
        data: to_binary(&ibc_packet_data)?,
        timeout: IbcTimeout::from(
            env.block
                .time
                .plus_seconds(DEFAULT_TIMEOUT_TIMESTAMP_OFFSET),
        ),
    };

    let res = Response::default()
        .add_message(ibc_msg)
        .add_attribute("pool_id", msg.pool_id.clone())
        .add_attribute("action", "take_pool");
    Ok(res)
}

pub fn single_asset_deposit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: MsgSingleAssetDepositRequest,
) -> Result<Response, ContractError> {

    if let Err(err) = msg.validate_basic() {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Failed to validate message: {}",
            err
        ))));
    }

    // check if given tokens are received here
    let mut ok = false;
    for asset in info.funds {
        if asset.denom == msg.token.denom && asset.amount == msg.token.amount {
            ok = true;
        }
    }
    if !ok {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Funds mismatch: Funds mismatched to with message and sent values: Take Pool"
        ))));
    }

    let pool_id = msg.pool_id.clone();
    let pool = POOLS.load(deps.storage, &pool_id)?;

    // If the pool is empty, then return a `Failure` response
    if pool.supply.amount.is_zero() {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Single asset cannot be provided to empty pool"
        ))));
    }

    if pool.status != PoolStatus::Active {
        return Err(ContractError::NotReadyForSwap);
    }

    // Create the interchain market maker (amm).
    let amm = InterchainMarketMaker {
       // pool_id: pool_id.clone(),
        pool: pool.clone(),
        fee_rate: pool.swap_fee,
    };

    // Deposit single asset to the AMM.
    let pool_token = amm
        .deposit_single_asset(&msg.token)
        .map_err(|err| StdError::generic_err(format!("Failed to deposit single asset: {}", err)))?;

    let msg_data = to_binary(&msg).unwrap();
    let state_change_data = to_binary(&StateChange {
        in_tokens: None,
        out_tokens: None,
        pool_tokens: Some(vec![pool_token]),
        pool_id: None,
        multi_deposit_order_id: None,
        source_chain_id: None,
    })?;
    // Construct the IBC swap packet.
    let packet_data = InterchainSwapPacketData {
        r#type: InterchainMessageType::SingleAssetDeposit,
        data: msg_data, // Use proper serialization for the `data` field.
        state_change: Some(state_change_data),
    };

    // Send the IBC swap packet.
    // if that relayer is died, so can't recover that port and channel so have to use new relayer?  
    let ibc_msg = IbcMsg::SendPacket {
        channel_id: pool.counter_party_channel.clone(),
        data: to_binary(&packet_data)?,
        timeout: IbcTimeout::from(
            env.block
                .time
                .plus_seconds(DEFAULT_TIMEOUT_TIMESTAMP_OFFSET),
        ),
    };

    let res = Response::default()
        .add_message(ibc_msg)
        .add_attribute("pool_id", msg.pool_id.clone())
        .add_attribute("action", "single_asset_deposit");
    Ok(res)
}

fn make_multi_asset_deposit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: MsgMakeMultiAssetDepositRequest,
) -> Result<Response, ContractError> {
   // load pool throw error if not found
   let interchain_pool_temp = POOLS.may_load(deps.storage, &msg.pool_id)?;
   let interchain_pool;
   if let Some(pool) = interchain_pool_temp {
       interchain_pool = pool
   } else {
       return Err(ContractError::Std(StdError::generic_err(format!(
           "Pool doesn't exist {}", msg.pool_id
       ))));
   }
   // TODO: deposit balance or any balance can't be zero
   // Add checks in every function

   let mut tokens: [Coin; 2] = Default::default();
   tokens[0] = msg.deposits[0].balance.clone();
   tokens[1] = msg.deposits[1].balance.clone();

    // check if given tokens are received here
    let mut ok = false;
    // First token in this chain only first token needs to be verified
    for asset in info.funds {
        if asset.denom == tokens[0].denom && asset.amount == tokens[0].amount ||
        (asset.denom == tokens[1].denom && asset.amount == tokens[1].amount) {
            ok = true;
        }
    }
    if !ok {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Funds mismatch: Funds mismatched to with message and sent values: Make Pool"
        ))));
    }

    // Check the pool status
    if interchain_pool.status != PoolStatus::Active {
        return Err(ContractError::NotReadyForSwap);
    }

    // Create the interchain market maker
    let amm = InterchainMarketMaker {
        pool: interchain_pool.clone(),
        fee_rate: interchain_pool.swap_fee,
    };

    // Deposit the assets into the interchain market maker
    let pool_tokens = amm.deposit_multi_asset(&vec![
        msg.deposits[0].balance.clone(),
        msg.deposits[1].balance.clone(),
    ])?;

    let mut config = CONFIG.load(deps.storage)?;
    config.counter = config.counter + 1;
    
    let multi_asset_order = MultiAssetDepositOrder {
        id: get_order_id(msg.deposits[0].sender.clone(), config.counter).to_string(),
        chain_id: msg.chain_id.clone(),
        pool_id: msg.pool_id.clone(),
        source_maker: msg.deposits[0].sender.clone(),
        destination_taker: msg.deposits[1].sender.clone(),
        deposits: get_coins_from_deposits(msg.deposits.clone()),
        //pool_tokens: pool_tokens,
        status: OrderStatus::Pending,
        created_at: env.block.height
    };

    // load orders
    // check for order, if exist throw error.

    let ac_key = msg.deposits[0].sender.clone() + "-" + &msg.pool_id.clone() + "-" + &msg.deposits[1].sender.clone();

    // save order in source chain
    let key = msg.pool_id.clone() + "-" + &multi_asset_order.id.clone().to_string();
    MULTI_ASSET_DEPOSIT_ORDERS.save(deps.storage, key, &multi_asset_order)?;
    ACTIVE_ORDERS.save(deps.storage, ac_key, &multi_asset_order)?;
    CONFIG.save(deps.storage, &config)?;

    // Construct the IBC packet
    let state_change_data = to_binary(&StateChange {
        in_tokens: None,
        out_tokens: None,
        pool_tokens: Some(pool_tokens),
        pool_id: None,
        multi_deposit_order_id: Some(multi_asset_order.id),
        source_chain_id: None,
    })?;
    let packet_data = InterchainSwapPacketData {
        r#type: InterchainMessageType::MakeMultiDeposit,
        data: to_binary(&msg.clone())?,
        state_change: Some(state_change_data),
    };

    // TODO: if that relayer is died, so can't recover that port and channel so have to use new relayer? 
    let ibc_msg = IbcMsg::SendPacket {
        channel_id: interchain_pool.clone().counter_party_channel,
        data: to_binary(&packet_data)?,
        timeout: IbcTimeout::from(
            env.block
                .time
                .plus_seconds(DEFAULT_TIMEOUT_TIMESTAMP_OFFSET),
        ),
    };

    let res = Response::default()
        .add_message(ibc_msg)
        .add_attribute("pool_id", msg.pool_id.clone())
        .add_attribute("action", "make_multi_asset_deposit");
    Ok(res)
}

fn cancel_multi_asset_deposit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: MsgCancelMultiAssetDepositRequest,
) -> Result<Response, ContractError> {
    // load pool throw error if not found
    let interchain_pool_temp = POOLS.may_load(deps.storage, &msg.pool_id)?;
    let interchain_pool;
    if let Some(pool) = interchain_pool_temp {
        interchain_pool = pool
    } else {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Pool doesn't exist {}", msg.pool_id
        ))));
    }
    // get order
    // load orders
    let key = msg.pool_id.clone() + "-" + &msg.order_id.clone().to_string();
    let multi_asset_order_temp = MULTI_ASSET_DEPOSIT_ORDERS.may_load(deps.storage, key)?;
    let multi_asset_order;
    if let Some(order) = multi_asset_order_temp {
        multi_asset_order = order;
    } else {
        return Err(ContractError::ErrOrderNotFound);
    }

    if multi_asset_order.source_maker != info.sender {
        return Err(ContractError::InvalidSender);
    }

    if multi_asset_order.status != OrderStatus::Pending {
        return Err(ContractError::ErrOrderAlreadyCompleted);
    }

    let packet_data = InterchainSwapPacketData {
        r#type: InterchainMessageType::CancelMultiDeposit,
        data: to_binary(&msg.clone())?,
        state_change: None,
    };
 
    let ibc_msg = IbcMsg::SendPacket {
        channel_id: interchain_pool.clone().counter_party_channel,
        data: to_binary(&packet_data)?,
        timeout: IbcTimeout::from(
            env.block
                .time
                .plus_seconds(DEFAULT_TIMEOUT_TIMESTAMP_OFFSET),
        ),
    };

    let res = Response::default()
        .add_message(ibc_msg)
        .add_attribute("pool_id", msg.pool_id.clone())
        .add_attribute("action", "cancel_multi_asset_deposit");
    Ok(res)
}

fn take_multi_asset_deposit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: MsgTakeMultiAssetDepositRequest,
) -> Result<Response, ContractError> {
    // load pool throw error if not found
    let interchain_pool_temp = POOLS.may_load(deps.storage, &msg.pool_id)?;
    let interchain_pool;
    if let Some(pool) = interchain_pool_temp {
        interchain_pool = pool
    } else {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Pool doesn't exist {}", msg.pool_id
        ))));
    }
    // get order
    // load orders
    let key = msg.pool_id.clone() + "-" + &msg.order_id.clone().to_string();
    let multi_asset_order_temp = MULTI_ASSET_DEPOSIT_ORDERS.may_load(deps.storage, key)?;
    let multi_asset_order;
    if let Some(order) = multi_asset_order_temp {
        multi_asset_order = order;
    } else {
        return Err(ContractError::ErrOrderNotFound);
    }

    if multi_asset_order.destination_taker != info.sender {
        return Err(ContractError::ErrFailedMultiAssetDeposit);
    }

    if multi_asset_order.status == OrderStatus::Complete {
        return Err(ContractError::ErrOrderAlreadyCompleted);
    }

    let token = interchain_pool.find_asset_by_side(PoolSide::SOURCE)
    .map_err(|err| StdError::generic_err(format!("Failed to find asset: {}", err)))?;
    // check if given tokens are received here
    let mut ok = false;
    // First token in this chain only first token needs to be verified
    for asset in info.funds {
        if asset.denom == token.balance.denom && multi_asset_order.deposits[1].amount == asset.amount 
        && asset.denom == multi_asset_order.deposits[1].denom {
            ok = true;
        }
    }
    if !ok {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Funds mismatch: Funds mismatched to with message and sent values: Take Multi Asset"
        ))));
    }

    // find number of tokens to be minted
    // Create the interchain market maker (amm).
    let amm = InterchainMarketMaker {
        pool: interchain_pool.clone(),
        fee_rate: interchain_pool.swap_fee,
    };

    let pool_tokens = amm.deposit_multi_asset(&multi_asset_order.deposits)?;

    // Construct the IBC packet
    let state_change_data = to_binary(&StateChange {
        in_tokens: None,
        out_tokens: None,
        pool_tokens: Some(pool_tokens),
        pool_id: None,
        multi_deposit_order_id: None,
        source_chain_id: None,
    })?;
    let packet_data = InterchainSwapPacketData {
        r#type: InterchainMessageType::TakeMultiDeposit,
        data: to_binary(&msg.clone())?,
        state_change: Some(state_change_data),
    };

    let ibc_msg = IbcMsg::SendPacket {
        channel_id: interchain_pool.clone().counter_party_channel,
        data: to_binary(&packet_data)?,
        timeout: IbcTimeout::from(
            env.block
                .time
                .plus_seconds(DEFAULT_TIMEOUT_TIMESTAMP_OFFSET),
        ),
    };

    let res = Response::default()
        .add_message(ibc_msg)
        .add_attribute("pool_id", msg.pool_id.clone())
        .add_attribute("action", "take_multi_asset_deposit");
    Ok(res)
}

// Pass pool id asset i.e cw20
fn multi_asset_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: MsgMultiAssetWithdrawRequest,
) -> Result<Response, ContractError> {
    // Get liquidity pool
    // load pool throw error if not found
    let interchain_pool_temp = POOLS.may_load(deps.storage, &msg.pool_id.clone())?;
    let interchain_pool;
    if let Some(pool) = interchain_pool_temp {
        interchain_pool = pool
    } else {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Pool doesn't exist {}", msg.pool_id
        ))));
    }

    let sub_messages: Vec<SubMsg>;
    if let Some(lp_token) = POOL_TOKENS_LIST.may_load(deps.storage, &msg.pool_id.clone())? {
         // Transfer tokens from user account to contract
        let msg = Cw20ExecuteMsg::TransferFrom { 
            owner: info.sender.to_string().clone(), 
            recipient: env.contract.address.to_string().clone(),
            amount: msg.pool_token.amount
        };
        let exec = WasmMsg::Execute {
            contract_addr: lp_token.into(),
            msg: to_binary(&msg)?,
            funds: vec![],
        };
        sub_messages = vec![SubMsg::new(exec)];
    } else {
        // throw error token not found, initialization is done in make_pool and
        // take_pool
        return Err(ContractError::Std(StdError::generic_err(format!(
            "LP Token is not initialized"
        ))));
    }

    // Create the interchain market maker
    let amm = InterchainMarketMaker {
        pool: interchain_pool.clone(),
        fee_rate: interchain_pool.swap_fee,
    };

    let refund_assets = amm.multi_asset_withdraw(msg.pool_token.clone())
    .map_err(|err| StdError::generic_err(format!("Failed to withdraw multi asset: {}", err)))?;

    let source_denom = interchain_pool.find_asset_by_side(PoolSide::SOURCE)
    .map_err(|err| StdError::generic_err(format!("Failed to find asset: {}", err)))?;

    let destination_denom = interchain_pool.find_asset_by_side(PoolSide::DESTINATION)
    .map_err(|err| StdError::generic_err(format!("Failed to find asset: {}", err)))?;

    let mut source_out = Coin { denom: "mock".to_string(), amount: Uint128::zero()};
    let mut destination_out = Coin { denom: "mock".to_string(), amount: Uint128::zero()};

    for asset in refund_assets {
        if &asset.denom == &source_denom.balance.denom {
            source_out = asset.clone();
        }
        if &asset.denom == &destination_denom.balance.denom {
            destination_out = asset;
        }
    }

    let state_change_data = to_binary(&StateChange {
        in_tokens: Some(vec![
            msg.pool_token.clone()
         ]),
        out_tokens: Some(vec![source_out, destination_out]),
        pool_tokens: Some(vec![
            msg.pool_token.clone()
        ]),
        pool_id: None,
        multi_deposit_order_id: None,
        source_chain_id: None,
    })?;

    let packet = InterchainSwapPacketData {
        r#type: InterchainMessageType::MultiWithdraw,
        data: to_binary(&msg)?,
        state_change: Some(state_change_data),
    };

    let ibc_msg = IbcMsg::SendPacket {
        channel_id: interchain_pool.counter_party_channel,
        data: to_binary(&packet)?,
        timeout: IbcTimeout::from(
            env.block
                .time
                .plus_seconds(DEFAULT_TIMEOUT_TIMESTAMP_OFFSET),
        ),
    };

    let res = Response::default()
        .add_submessages(sub_messages)
        .add_message(ibc_msg)
        .add_attribute("pool_id", msg.pool_id.clone())
        .add_attribute("action", "multi_asset_withdraw");
    Ok(res)
}

fn swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: MsgSwapRequest,
) -> Result<Response, ContractError> {
    // Get liquidity pool
    // load pool throw error if not found
    let interchain_pool_temp = POOLS.may_load(deps.storage, &msg.pool_id.clone())?;
    let interchain_pool;
    if let Some(pool) = interchain_pool_temp {
        interchain_pool = pool
    } else {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Pool doesn't exist {}", msg.pool_id
        ))));
    }

    // Check the pool status
    if interchain_pool.status != PoolStatus::Active {
        return Err(ContractError::NotReadyForSwap);
    }

    // check if given tokens are received here
    let mut ok = false;
    // First token in this chain only first token needs to be verified
    for asset in info.funds {
        if asset.denom == msg.token_in.denom && asset.amount == msg.token_in.amount {
            ok = true;
        }
    }
    if !ok {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Funds mismatch: Funds mismatched to with message and sent values: Swap"
        ))));
    }

    // Create the interchain market maker
    let amm = InterchainMarketMaker {
        pool: interchain_pool.clone(),
        fee_rate: interchain_pool.swap_fee,
    };

    // Construct the IBC data packet
    let swap_data = to_binary(&msg)?;
    let token_out: Coin;
    let msg_type: InterchainMessageType;

    match msg.swap_type {
        SwapMsgType::LEFT => {
            msg_type = InterchainMessageType::LeftSwap;
            token_out = amm.compute_swap(msg.token_in.clone(), &msg.token_out.denom)?;
        }
        SwapMsgType::RIGHT => {
            msg_type = InterchainMessageType::RightSwap;
            token_out = amm.compute_offer_amount(msg.token_in.clone(), msg.token_out.clone())?;
        }
    }

    // Slippage checking
    let factor = MAXIMUM_SLIPPAGE - msg.slippage;
    let expected = msg
        .token_out
        .amount
        .mul(Uint128::from(factor))
        .div(Uint128::from(MAXIMUM_SLIPPAGE));
    if token_out.amount.lt(&expected) {
        return Err(ContractError::FailedOnSwapReceived {
            err: format!(
                "slippage check failed! expected: {}, output: {:?}, factor: {}",
                expected, token_out, factor
            ),
        });
    }

    let state_change_data = to_binary(&StateChange {
        in_tokens: None,
        out_tokens: Some(vec![token_out]),
        pool_tokens: None,
        pool_id: None,
        multi_deposit_order_id: None,
        source_chain_id: None,
    })?;
    let packet = InterchainSwapPacketData {
        r#type: msg_type,
        data: swap_data,
        state_change: Some(state_change_data),
    };

    let ibc_msg = IbcMsg::SendPacket {
        channel_id: interchain_pool.counter_party_channel,
        data: to_binary(&packet)?,
        timeout: IbcTimeout::from(
            env.block
                .time
                .plus_seconds(DEFAULT_TIMEOUT_TIMESTAMP_OFFSET),
        ),
    };

    let res = Response::default()
        .add_message(ibc_msg)
        .add_attribute("pool_id", msg.pool_id.clone())
        .add_attribute("action", "swap");
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::InterchainPool { pool_id } => to_binary(&query_interchain_pool(deps, pool_id)?),
        QueryMsg::InterchainPoolList {  start_after, limit } => 
            to_binary(&query_interchain_pool_list(deps, start_after, limit)?),
        QueryMsg::Order { pool_id, order_id } => 
            to_binary(&query_order(deps, pool_id, order_id)?),
        QueryMsg::OrderList { start_after, limit } =>
            to_binary(&query_orders(deps, start_after, limit)?),
        QueryMsg::PoolAddressByToken { pool_id } => to_binary(&query_pool_address(deps, pool_id)?),
        QueryMsg::PoolTokenList { start_after, limit } =>
            to_binary(&query_pool_list(deps, start_after, limit)?),
        QueryMsg::LeftSwap { pool_id, token_in, token_out } =>
            to_binary(&query_left_swap(deps, pool_id, token_in, token_out)?),
        QueryMsg::RightSwap { pool_id, token_in, token_out } =>
        to_binary(&query_right_swap(deps, pool_id, token_in, token_out)?),
        QueryMsg::QueryActiveOrders { source_maker, destination_taker ,pool_id } =>
        to_binary(&query_active_orders(deps, pool_id, source_maker, destination_taker)?),
        QueryMsg::Rate { pool_id, amount } => to_binary(&query_rate(deps, pool_id, amount)?),
    }
}

/// Settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

fn query_config(
    deps: Deps,
) -> StdResult<QueryConfigResponse> {
    let config = CONFIG.load(deps.storage)?;

    Ok(QueryConfigResponse { counter: config.counter, token_code_id: config.token_code_id })
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

fn query_interchain_pool(
    deps: Deps,
    pool_id: String
) -> StdResult<InterchainPoolResponse> {
    // load pool throw error if found
    let interchain_pool_temp = POOLS.may_load(deps.storage, &pool_id)?;
    let interchain_pool;
    if let Some(pool) = interchain_pool_temp {
        interchain_pool = pool;
    } else {
        return Err(StdError::generic_err(format!(
            "Pool not found"
        )));
    }

    Ok(InterchainPoolResponse {
        id: interchain_pool.id,
        source_creator: interchain_pool.source_creator,
        destination_creator: interchain_pool.destination_creator,
        assets: interchain_pool.assets,
        swap_fee: interchain_pool.swap_fee,
        supply: interchain_pool.supply,
        status: interchain_pool.status,
        counter_party_channel: interchain_pool.counter_party_channel,
        counter_party_port: interchain_pool.counter_party_port,
        source_chain_id: interchain_pool.source_chain_id,
        destination_chain_id: interchain_pool.destination_chain_id
    })
}

fn query_interchain_pool_list(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<InterchainListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|denom| Bound::ExclusiveRaw(denom.into_bytes()));
    let list = POOLS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item: Result<(String, InterchainLiquidityPool), cosmwasm_std::StdError>| item.unwrap().1)
        .collect::<Vec<InterchainLiquidityPool>>();

    Ok(InterchainListResponse { pools: list })
}

fn query_order(
    deps: Deps,
    pool_id: String,
    order_id: String
) -> StdResult<MultiAssetDepositOrder> {
    let key = pool_id + "-" + &order_id;
    let multi_asset_order_temp = MULTI_ASSET_DEPOSIT_ORDERS.may_load(deps.storage, key)?;
    let multi_asset_order;
    if let Some(order) = multi_asset_order_temp {
        multi_asset_order = order;
    } else {
        return Err(StdError::generic_err(format!(
            "Order not found"
        )));
    };

    Ok(multi_asset_order)
}

fn query_orders(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<OrderListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|denom| Bound::ExclusiveRaw(denom.into_bytes()));
    let list = MULTI_ASSET_DEPOSIT_ORDERS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item: Result<(String, MultiAssetDepositOrder), cosmwasm_std::StdError>| item.unwrap().1)
        .collect::<Vec<MultiAssetDepositOrder>>();

    Ok(OrderListResponse { orders: list })
}

fn query_pool_address(
    deps: Deps,
    pool_id: String
) -> StdResult<String> {
    let res;
    if let Some(lp_token) = POOL_TOKENS_LIST.may_load(deps.storage, &pool_id.clone())? {
        res = lp_token
    } else {
        // throw error token not found, initialization is done in make_pool and
        // take_pool
        return Err(StdError::generic_err(format!(
            "LP Token is not initialized"
        )));
    }

    Ok(res)
}

fn query_pool_list(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<PoolListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|denom| Bound::ExclusiveRaw(denom.into_bytes()));
    let list = POOL_TOKENS_LIST
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item: Result<(String, String), cosmwasm_std::StdError>| item.unwrap().1)
        .collect::<Vec<String>>();

    Ok(PoolListResponse { pools: list })
}

fn query_left_swap(
    deps: Deps,
    pool_id: String,
    token_in: Coin,
    token_out: Coin
) -> StdResult<Coin> {
    // Get liquidity pool
    // load pool throw error if not found
    let interchain_pool_temp = POOLS.may_load(deps.storage, &pool_id.clone())?;
    let interchain_pool;
    if let Some(pool) = interchain_pool_temp {
        interchain_pool = pool
    } else {
        return Err(StdError::generic_err(format!(
            "Pool doesn't exist {}", pool_id
        )));
    }

    // Check the pool status
    if interchain_pool.status != PoolStatus::Active {
        return Err(StdError::generic_err(format!(
            "Pool not ready for swap!"
        )));
    }

    // Create the interchain market maker
    let amm = InterchainMarketMaker {
        //pool_id: interchain_pool.clone().id,
        pool: interchain_pool.clone(),
        fee_rate: interchain_pool.swap_fee,
    };
    let result = amm.compute_swap(token_in.clone(), &token_out.denom)?;
    Ok(result)
}

fn query_right_swap(
    deps: Deps,
    pool_id: String,
    token_in: Coin,
    token_out: Coin
) -> StdResult<Coin> {
    // Get liquidity pool
    // load pool throw error if not found
    let interchain_pool_temp = POOLS.may_load(deps.storage, &pool_id.clone())?;
    let interchain_pool;
    if let Some(pool) = interchain_pool_temp {
        interchain_pool = pool
    } else {
        return Err(StdError::generic_err(format!(
            "Pool doesn't exist {}", pool_id
        )));
    }

    // Check the pool status
    if interchain_pool.status != PoolStatus::Active {
        return Err(StdError::generic_err(format!(
            "Pool not ready for swap!"
        )));
    }

    // Create the interchain market maker
    let amm = InterchainMarketMaker {
        //pool_id: interchain_pool.clone().id,
        pool: interchain_pool.clone(),
        fee_rate: interchain_pool.swap_fee,
    };
    let result = amm.compute_offer_amount(token_in.clone(), token_out)?;
    Ok(result)
}

fn query_active_orders(
    deps: Deps,
    pool_id: String,
    source_maker: String,
    destination_taker: String
) -> StdResult<MultiAssetDepositOrder> {
    let key = source_maker + "-" + &pool_id + "-" + &destination_taker;
    let multi_asset_order_temp = ACTIVE_ORDERS.may_load(deps.storage, key)?;
    let multi_asset_order;
    if let Some(order) = multi_asset_order_temp {
        multi_asset_order = order;
    } else {
        return Err(StdError::generic_err(format!(
            "No active order"
        )));
    };

    Ok(multi_asset_order)
}

fn query_rate(deps: Deps, pool_id: String, amount: Uint128) -> StdResult<Vec<Coin>> {
    // Get liquidity pool
    // load pool throw error if not found
    let interchain_pool_temp = POOLS.may_load(deps.storage, &pool_id)?;
    let interchain_pool;
    if let Some(pool) = interchain_pool_temp {
        interchain_pool = pool
    } else {
        return Err(StdError::generic_err(format!(
            "Pool doesn't exist {}", pool_id
        )));
    }

    // Create the interchain market maker
    let amm = InterchainMarketMaker {
        pool: interchain_pool.clone(),
        fee_rate: interchain_pool.swap_fee,
    };

    Ok(amm.multi_asset_withdraw(Coin {amount: amount, denom: pool_id})?)
}


#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();

        // Instantiate an empty contract
        let instantiate_msg = InstantiateMsg { token_code_id: 1 };
        let info = mock_info("anyone", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());
    }
}
