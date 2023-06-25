use std::ops::{Div, Mul};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Coin, DepsMut, Env, IbcMsg, IbcTimeout, MessageInfo, Response, StdError, StdResult,
    Uint128,
};

use cw2::set_contract_version;

use crate::error::ContractError;
use crate::market::{InterchainMarketMaker, PoolStatus, PoolSide};
use crate::msg::{
    ExecuteMsg, InstantiateMsg,
    MsgMultiAssetWithdrawRequest, MsgSingleAssetDepositRequest,
    MsgSwapRequest, SwapMsgType, MsgMakePoolRequest, MsgTakePoolRequest, MsgMakeMultiAssetDepositRequest, MsgTakeMultiAssetDepositRequest,
};
use crate::state::{POOLS, MULTI_ASSET_DEPOSIT_ORDERS, CONFIG};
use crate::types::{InterchainSwapPacketData, StateChange, InterchainMessageType, MultiAssetDepositOrder, OrderStatus, MULTI_DEPOSIT_PENDING_LIMIT};
use crate::utils::{check_slippage, get_coins_from_deposits};

// Version info, for migration info
const CONTRACT_NAME: &str = "ics101-interchainswap";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_TIMEOUT_TIMESTAMP_OFFSET: u64 = 600;
//const MAX_FEE_RATE: u32 = 300;
const MAXIMUM_SLIPPAGE: u64 = 10000;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    // No setup
    // TODO: add counter and token id to state
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
        ExecuteMsg::MakePool(msg) => make_pool(deps, env, info, msg),
        ExecuteMsg::TakePool(msg) => take_pool(deps, env, info, msg),
        ExecuteMsg::SingleAssetDeposit(msg) => single_asset_deposit(deps, env, info, msg),
        ExecuteMsg::MakeMultiAssetDeposit(msg) => make_multi_asset_deposit(deps, env, info, msg),
        ExecuteMsg::TakeMultiAssetDeposit(msg) => take_multi_asset_deposit(deps, env, info, msg),
        ExecuteMsg::MultiAssetWithdraw(msg) => multi_asset_withdraw(deps, env, info, msg),
        ExecuteMsg::Swap(msg) => swap(deps, env, info, msg),
    }
}

fn make_pool(
    _deps: DepsMut,
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

    // check if given tokens are received here
    let mut ok = false;
    // First token in this chain only first token needs to be verified
    for asset in info.funds {
        if asset.denom == tokens[0].denom && asset.amount == tokens[0].amount {
            ok = true;
        }
    }
    if !ok {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Funds mismatch: Funds mismatched to with message and sent values: Make Pool"
        ))));
    }

    let pool_data = to_binary(&msg).unwrap();
    let ibc_packet_data = InterchainSwapPacketData {
        r#type: InterchainMessageType::MakePool,
        data: pool_data.clone(),
        state_change: None,
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
        .add_message(ibc_msg)
        .add_attribute("action", "make_pool");
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
    // check balance and funds sent handle error
    // TODO: Handle unwrap
    let token = interchain_pool.find_asset_by_side(PoolSide::SOURCE).unwrap();
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
        .add_attribute("action", "make_pool");
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

    if pool.status != PoolStatus::PoolStatusActive {
        return Err(ContractError::NotReadyForSwap);
    }

    // Create the interchain market maker (amm).
    let amm = InterchainMarketMaker {
        pool_id: pool_id.clone(),
        pool: pool.clone(),
        fee_rate: pool.swap_fee,
    };

    // Deposit single asset to the AMM.
    let pool_token = amm
        .deposit_single_asset(&msg.token)
        .map_err(|err| StdError::generic_err(format!("Failed to deposit single asset: {}", err)))?;

    let msg_data = to_binary(&msg).unwrap();
    // Construct the IBC swap packet.
    let packet_data = InterchainSwapPacketData {
        r#type: InterchainMessageType::SingleAssetDeposit,
        data: msg_data, // Use proper serialization for the `data` field.
        state_change: Some(StateChange {
            in_tokens: None,
            out_tokens: None,
            pool_tokens: Some(vec![pool_token]),
        }),
    };

    // Send the IBC swap packet.
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

   let mut tokens: [Coin; 2] = Default::default();
   tokens[0] = msg.deposits[0].balance.clone();
   tokens[1] = msg.deposits[1].balance.clone();

    // check if given tokens are received here
    let mut ok = false;
    // First token in this chain only first token needs to be verified
    for asset in info.funds {
        if asset.denom == tokens[0].denom && asset.amount == tokens[0].amount {
            ok = true;
        }
    }
    if !ok {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Funds mismatch: Funds mismatched to with message and sent values: Make Pool"
        ))));
    }

    // Check the pool status
    if interchain_pool.status != PoolStatus::PoolStatusActive {
        return Err(ContractError::NotReadyForSwap);
    }

    // TODO: Handle unwrap
    let source_asset = interchain_pool.find_asset_by_side(PoolSide::SOURCE).unwrap();
    let destination_asset = interchain_pool.find_asset_by_side(PoolSide::DESTINATION).unwrap();

    // Check the ratio of local amount and remote amount
    let current_ratio = Uint128::from(source_asset.balance.amount)
        .mul(Uint128::from(1e18 as u64))
        .div(Uint128::from(destination_asset.balance.amount));
    let input_ratio = Uint128::from(msg.deposits[0].balance.amount)
        .mul(Uint128::from(1e18 as u64))
        .div(Uint128::from(msg.deposits[1].balance.amount));

    check_slippage(current_ratio, input_ratio, 10)
        .map_err(|err| StdError::generic_err(format!("Invalid Slippage: {}", err)))?;

    // Create the interchain market maker
    let amm = InterchainMarketMaker {
        pool_id: interchain_pool.clone().pool_id,
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

    let mut multi_asset_order = MultiAssetDepositOrder {
        order_id: config.counter,
        pool_id: msg.pool_id.clone(),
        source_maker: msg.deposits[0].sender.clone(),
        destination_taker: msg.deposits[1].sender.clone(),
        deposits: get_coins_from_deposits(msg.deposits.clone()),
        //pool_tokens: pool_tokens,
        status: OrderStatus::Pending,
        created_at: env.block.height
    };

    // load orders
    let mut multi_asset_orders: Vec<MultiAssetDepositOrder> = MULTI_ASSET_DEPOSIT_ORDERS.load(deps.storage, msg.pool_id.clone())?;
    let mut found = false;
    if multi_asset_orders.len() > 0 {
        found = true;
        config.counter = config.counter - 1;
        // we already checked for vector length
        multi_asset_order = multi_asset_orders.last().unwrap().clone();
    }

    let pending_height = env.block.height - multi_asset_order.created_at;
    if found && multi_asset_order.status == OrderStatus::Pending && pending_height < MULTI_DEPOSIT_PENDING_LIMIT {
        return Err(ContractError::ErrPreviousOrderNotCompleted);
    }

	// protect malicious deposit action. we will not refund token as a penalty.
    if found && pending_height > MULTI_DEPOSIT_PENDING_LIMIT {
        multi_asset_orders.pop();
    }

    // save order in source chain
    multi_asset_orders.push(multi_asset_order);
    MULTI_ASSET_DEPOSIT_ORDERS.save(deps.storage, msg.pool_id.clone(), &multi_asset_orders)?;
    CONFIG.save(deps.storage, &config)?;

    // Construct the IBC packet
    let packet_data = InterchainSwapPacketData {
        r#type: InterchainMessageType::MakeMultiDeposit,
        data: to_binary(&msg.clone())?,
        state_change: Some(StateChange {
            pool_tokens: Some(pool_tokens),
            in_tokens: None,
            out_tokens: None,
        }),
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
        .add_attribute("action", "make_multi_asset_deposit");
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
    let multi_asset_orders: Vec<MultiAssetDepositOrder> = MULTI_ASSET_DEPOSIT_ORDERS.load(deps.storage, msg.pool_id.clone())?;
    let mut found = false;
    let mut order = MultiAssetDepositOrder {
        order_id: 0,
        pool_id: "Mock".to_string(),
        source_maker: "Mock".to_string(),
        destination_taker: "Mock".to_string(),
        deposits: vec![],
        status: OrderStatus::Pending,
        created_at: env.block.height
    };
    for  multi_order in multi_asset_orders {
        if multi_order.order_id == msg.order_id {
            found = true;
            order = multi_order
        }
    }

    if !found {
        return Err(ContractError::ErrOrderNotFound);
    }

    if order.destination_taker != info.sender {
        return Err(ContractError::ErrFailedMultiAssetDeposit);
    }

    // TODO: Add chain id to order and add check
    // TODO: Make sure the pool side, i think it will be destination .. Handle erorr
    let token = interchain_pool.find_asset_by_side(PoolSide::DESTINATION).unwrap();
    // check if given tokens are received here
    let mut ok = false;
    // First token in this chain only first token needs to be verified
    for asset in info.funds {
        if asset.denom == token.balance.denom && asset.amount == token.balance.amount {
            ok = true;
        }
    }
    if !ok {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Funds mismatch: Funds mismatched to with message and sent values: Make Pool"
        ))));
    }
    // Construct the IBC packet
    let packet_data = InterchainSwapPacketData {
        r#type: InterchainMessageType::TakeMultiDeposit,
        data: to_binary(&msg.clone())?,
        state_change: Some(StateChange {
            pool_tokens: None,
            in_tokens: None,
            out_tokens: None,
        }),
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
        .add_attribute("action", "take_multi_asset_deposit");
    Ok(res)
}

// TODO: Call from receive function only
// Pass pool id asset i.e cw20
fn multi_asset_withdraw(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
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

    // Create the interchain market maker
    let amm = InterchainMarketMaker {
        pool_id: interchain_pool.clone().pool_id,
        pool: interchain_pool.clone(),
        fee_rate: interchain_pool.swap_fee,
    };

    // TODO: Handle unwrap
    let source_denom = interchain_pool.find_asset_by_side(PoolSide::SOURCE).unwrap();
    let source_out = amm.multi_asset_withdraw(Coin {
		denom: interchain_pool.pool_id.clone(), amount: msg.pool_token.amount.div(Uint128::from(2u64)),
	}, &source_denom.balance.denom).unwrap();
    let destination_denom = interchain_pool.find_asset_by_side(PoolSide::DESTINATION).unwrap();
    let destination_out = amm.multi_asset_withdraw(Coin {
		denom: interchain_pool.pool_id.clone(), amount: msg.pool_token.amount.div(Uint128::from(2u64)),
	}, &destination_denom.balance.denom).unwrap();

    let packet = InterchainSwapPacketData {
        r#type: InterchainMessageType::MultiWithdraw,
        data: to_binary(&msg)?,
        state_change: Some(StateChange {
            pool_tokens: Some(vec![
               msg.pool_token
            ]),
            in_tokens: None,
            out_tokens: Some(vec![source_out, destination_out]),
        }),
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
    if interchain_pool.status != PoolStatus::PoolStatusActive {
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
            "Funds mismatch: Funds mismatched to with message and sent values: Make Pool"
        ))));
    }

    // Create the interchain market maker
    let amm = InterchainMarketMaker {
        pool_id: interchain_pool.clone().pool_id,
        pool: interchain_pool.clone(),
        fee_rate: interchain_pool.swap_fee,
    };

    // Construct the IBC data packet
    let swap_data = to_binary(&msg)?;
    let token_out: Option<Coin>;
    let msg_type: Option<InterchainMessageType>;

    match msg.swap_type {
        SwapMsgType::Left => {
            msg_type = Some(InterchainMessageType::LeftSwap);
            token_out = Some(amm.left_swap(msg.token_in.clone(), &msg.token_out.denom)?);
        }
        SwapMsgType::Right => {
            msg_type = Some(InterchainMessageType::RightSwap);
            token_out = Some(amm.right_swap(msg.token_in.clone(), msg.token_out.clone())?);
        }
    }

    let token_out = match token_out {
        Some(token) => token,
        None => {
            return Err(ContractError::FailedSwap {
                err: "token_out not found".to_string(),
            })
        }
    };

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

    let packet = InterchainSwapPacketData {
        r#type: msg_type.ok_or(ContractError::FailedSwap {
            err: "msg_type not found".to_string(),
        })?,
        data: swap_data,
        state_change: Some(StateChange {
            in_tokens: None,
            out_tokens: Some(vec![token_out]),
            pool_tokens: None,
        }),
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
        .add_attribute("action", "swap");
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();

        // Instantiate an empty contract
        let instantiate_msg = InstantiateMsg {};
        let info = mock_info("anyone", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());
    }
}
