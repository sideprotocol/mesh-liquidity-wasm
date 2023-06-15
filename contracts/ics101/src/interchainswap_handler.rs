use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    error::ContractError,
    types::{IBCSwapPacketData, SwapMessageType, StateChange},
    state::{Status, POOLS},
    utils::{
        send_tokens, decode_create_pool_msg, get_pool_id_with_tokens,
    }, msg::{MsgCreatePoolRequest, MsgSingleAssetDepositRequest, MsgMultiAssetDepositRequest, MsgSingleAssetWithdrawRequest, MsgMultiAssetWithdrawRequest, MsgSwapRequest}
    ,market::{InterchainLiquidityPool, PoolStatus::{PoolStatusInitial, PoolStatusReady}, PoolAsset},
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
    let packet_data: IBCSwapPacketData = from_binary(&packet.data)?;

    match packet_data.r#type {
        // TODO: Add test for each operation
        SwapMessageType::Unspecified => {
            let res = IbcReceiveResponse::new()
                .set_ack(ack_success())
                .add_attribute("action", "receive")
                .add_attribute("success", "true");
            Ok(res)
        }
        // Save pool data
        SwapMessageType::CreatePool => {
            let msg: MsgCreatePoolRequest = decode_create_pool_msg(&packet_data.data.clone());
            on_received_create_pool(deps, env, packet, msg)
        }
        //
        SwapMessageType::SingleDeposit => {
            let msg: MsgSingleAssetDepositRequest = from_binary(&packet_data.data.clone())?;
            on_received_single_deposit(deps, env, packet, msg, packet_data.state_change.unwrap())
        }
        SwapMessageType::MultiDeposit => {
            let msg: MsgMultiAssetDepositRequest = from_binary(&packet_data.data.clone())?;
            on_received_multi_deposit(deps, env, packet, msg, packet_data.state_change.unwrap())
        }
        SwapMessageType::SingleWithdraw => {
            let msg: MsgSingleAssetWithdrawRequest = from_binary(&packet_data.data.clone())?;
            on_received_single_withdraw(deps, env, packet, msg, packet_data.state_change.unwrap())
        }
        SwapMessageType::MultiWithdraw => {
            let msg: MsgMultiAssetWithdrawRequest = from_binary(&packet_data.data.clone())?;
            on_received_multi_withdraw(deps, env, packet, msg)
        }
        SwapMessageType::LeftSwap => {
            let msg: MsgSwapRequest = from_binary(&packet_data.data.clone())?;
            on_received_left_swap(deps, env, packet, msg)
        }
        SwapMessageType::RightSwap => {
            let msg: MsgSwapRequest = from_binary(&packet_data.data.clone())?;
            on_received_right_swap(deps, env, packet, msg)
        }
    }
}

pub(crate) fn on_received_create_pool(
    deps: DepsMut,
    _env: Env,
    packet: &IbcPacket,
    msg: MsgCreatePoolRequest,
) -> Result<IbcReceiveResponse, ContractError> {
    // get pool asset from tokens and weight
    if let Err(err) = msg.validate_basic() {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "Failed to validate message: {}",
            err
        ))));
    }

    // construct assets
    if msg.tokens.len() != msg.weights.len() || msg.weights.len() != msg.decimals.len() {
        return Err(ContractError::InvalidAssetInput);
    }
    let mut construct_assets = vec![];
    for i in 0..msg.weights.len() {
        construct_assets.push(PoolAsset {
            // TODO: check if this token has supply in this chain using cosmwasm
            side: crate::market::PoolSide::REMOTE,
            balance: msg.tokens[i],
            weight: msg.weights[i],
            decimal: msg.decimals[i],
        })
    }

    let pool_id = get_pool_id_with_tokens(&msg.tokens);
    let supply: Coin = Coin {amount: Uint128::from(0u64), denom: pool_id};
    let interchain_pool: InterchainLiquidityPool = InterchainLiquidityPool {
        pool_id: pool_id,
        creator: msg.sender,
        assets: construct_assets, supply: supply, pool_price: 0.0, status: PoolStatusInitial,
        encounter_party_port: msg.source_port,
        encounter_party_channel: msg.source_channel
    };

    POOLS.save(deps.storage, &pool_id, &interchain_pool)?;

    let res = IbcReceiveResponse::new()
        .set_ack(ack_success())
        .add_attribute("action", "receive")
        .add_attribute("success", "true")
        .add_attribute("sucess", "create_pool_receive");

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

    let mut interchain_pool = POOLS.load(deps.storage, &msg.pool_id)?;

    // Check status and update states accordingly
    if interchain_pool.status == PoolStatusReady {
        // increase lp token mint amount
        interchain_pool.add_supply(state_change.pool_tokens.unwrap()[0]);

        // update pool tokens.
        if let Err(err) =interchain_pool.add_asset(msg.token) {
            return Err(ContractError::Std(StdError::generic_err(format!(
                "Failed to add asset: {}",
                err
            ))));
        }
    } else {
        // switch pool status to 'READY'
        interchain_pool.status = PoolStatusReady
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

pub(crate) fn on_received_multi_deposit(
    deps: DepsMut,
    _env: Env,
    packet: &IbcPacket,
    msg: MsgMultiAssetDepositRequest,
    state_change: StateChange
) -> Result<IbcReceiveResponse, ContractError> {
    // if let Err(err) = msg.validate_basic() {
    //     return Err(ContractError::Std(StdError::generic_err(format!(
    //         "Failed to validate message: {}",
    //         err
    //     ))));
    // }

    // TODO: How to get tokens on remote chain, these are denom balance in chain ?

    // Validate the message
	// if err := msg.ValidateBasic(); err != nil {
	// 	return nil, err
	// }

	// // Verify the sender's address
	// senderAcc := k.authKeeper.GetAccount(ctx, sdk.MustAccAddressFromBech32(msg.RemoteDeposit.Sender))
	// senderPrefix, _, err := bech32.Decode(senderAcc.GetAddress().String())
	// if err != nil {
	// 	return nil, err
	// }
	// if sdk.GetConfig().GetBech32AccountAddrPrefix() != senderPrefix {
	// 	return nil, errorsmod.Wrapf(types.ErrFailedDoubleDeposit, "first address has to be this chain address (%s)", err)
	// }

	// // Retrieve the liquidity pool
	// pool, found := k.GetInterchainLiquidityPool(ctx, msg.PoolId)
	// if !found {
	// 	return nil, errorsmod.Wrapf(types.ErrFailedDoubleDeposit, "%s", types.ErrNotFoundPool)
	// }

	// // Lock assets from senders to escrow account
	// escrowAccount := types.GetEscrowAddress(pool.EncounterPartyPort, pool.EncounterPartyChannel)

	// // Create a deposit message
	// sendMsg := banktypes.MsgSend{
	// 	FromAddress: senderAcc.GetAddress().String(),
	// 	ToAddress:   escrowAccount.String(),
	// 	Amount:      sdk.NewCoins(*msg.RemoteDeposit.Token),
	// }

	// // Recover original signed Tx.
	// deposit := types.RemoteDeposit{
	// 	Sequence: senderAcc.GetSequence(),
	// 	Sender:   msg.RemoteDeposit.Sender,
	// 	Token:    msg.RemoteDeposit.Token,
	// }
	// rawDepositTx, err := types.ModuleCdc.Marshal(&deposit)

	// if err != nil {
	// 	return nil, err
	// }

	// pubKey := senderAcc.GetPubKey()
	// isValid := pubKey.VerifySignature(rawDepositTx, msg.RemoteDeposit.Signature)

	// if !isValid {
	// 	return nil, errorsmod.Wrapf(types.ErrFailedDoubleDeposit, ":%s", types.ErrInvalidSignature)
	// }

	// _, err = k.executeDepositTx(ctx, &sendMsg)
	// if err != nil {
	// 	return nil, err
	// }

	// // Increase LP token mint amount
	// for _, token := range stateChange.PoolTokens {
	// 	pool.AddPoolSupply(*token)
	// }

	// // Update pool tokens or switch pool status to 'READY'
	// if pool.Status == types.PoolStatus_POOL_STATUS_READY {
	// 	pool.AddAsset(*msg.LocalDeposit.Token)
	// 	pool.AddAsset(*msg.RemoteDeposit.Token)
	// } else {
	// 	pool.Status = types.PoolStatus_POOL_STATUS_READY
	// }

	// // Mint voucher tokens for the sender
	// err = k.MintTokens(ctx, senderAcc.GetAddress(), *stateChange.PoolTokens[1])
	// if err != nil {
	// 	return nil, errorsmod.Wrapf(types.ErrFailedDoubleDeposit, ":%s", err)
	// }
	// // Save pool
	// k.SetInterchainLiquidityPool(ctx, pool)
	// return &types.MsgMultiAssetDepositResponse{
	// 	PoolTokens: stateChange.PoolTokens,
	//}, nil

    let mut interchain_pool = POOLS.load(deps.storage, &msg.pool_id)?;

    // Check status and update states accordingly
    if (interchain_pool.status == PoolStatusReady) {
        // increase lp token mint amount
        interchain_pool.add_supply(state_change.pool_tokens.unwrap()[0]);

        // update pool tokens.
        if let Err(err) =interchain_pool.add_asset(msg.token) {
            return Err(ContractError::Std(StdError::generic_err(format!(
                "Failed to add asset: {}",
                err
            ))));
        }
    } else {
        // switch pool status to 'READY'
        interchain_pool.status = PoolStatusReady
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

pub(crate) fn on_received_single_withdraw(
    deps: DepsMut,
    _env: Env,
    packet: &IbcPacket,
    msg: MsgSingleAssetWithdrawRequest,
    state_change: StateChange
) -> Result<IbcReceiveResponse, ContractError> {
    let mut interchain_pool = POOLS.load(deps.storage, &msg.pool_id)?;
	// Update pool status by subtracting the supplied pool coin and output token
	for poolAsset in state_change.out_tokens.unwrap() {
		interchain_pool.subtract_asset(poolAsset);
	}

	for poolToken in state_change.pool_tokens.unwrap() {
		interchain_pool.subtract_supply(poolToken);
	}

    // save pool.
    POOLS.save(deps.storage, &msg.pool_id, &interchain_pool)?;

    let res = IbcReceiveResponse::new()
    .set_ack(ack_success())
    .add_attribute("action", "receive")
    .add_attribute("success", "true")
    .add_attribute("sucess", "single_asset_withraw");
    //.add_attribute("out_token", state_change.out);

    Ok(res)
}

// update the balance stored on this (channel, denom) index
// acknowledgement
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
