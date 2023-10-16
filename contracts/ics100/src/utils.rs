use cosmwasm_std::{
    from_binary, Addr, BankMsg, Binary, Coin, Deps, IbcAcknowledgement, IbcChannel, IbcOrder,
    StdError, StdResult, SubMsg, Uint128,
};

use sha2::{Digest, Sha256};

use crate::{
    atomic_swap_handler::AtomicSwapPacketAcknowledgement,
    msg::{Height, MakeSwapMsg, MakeSwapMsgOutput, TakeSwapMsg, TakeSwapMsgOutput},
    state::FEE_INFO,
    ContractError,
};

const FEE_BASIS_POINT: u64 = 10000;

pub fn generate_order_id(order_path: &str) -> StdResult<String> {
    // Generate random bytes
    // Create the ID by combining the order_path and random bytes
    let res: Vec<u8> = [order_path.as_bytes()].concat();

    // Hash the result to create a fixed-length ID
    let hash = Sha256::digest(&res);
    let id = hex::encode(hash);

    Ok(id)
}

pub fn order_path(
    source_channel: String,
    source_port: String,
    destination_channel: String,
    destination_port: String,
    sequence: u64,
) -> StdResult<String> {
    let path = format!(
        "channel/{}/port/{}/channel/{}/port/{}/{}",
        source_channel, source_port, destination_channel, destination_port, sequence
    );
    Ok(path)
}

pub fn extract_source_channel_for_taker_msg(path: &str) -> StdResult<String> {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() < 6 {
        return Err(StdError::generic_err("Invalid path"));
    }
    Ok(parts[5].to_string())
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

pub const ICS100_VERSION: &str = "ics100-1";
pub const ICS100_ORDERING: IbcOrder = IbcOrder::Unordered;

pub(crate) fn enforce_order_and_version(
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

pub(crate) fn decode_take_swap_msg(data: &Binary) -> TakeSwapMsg {
    let msg_res: Result<TakeSwapMsg, StdError> = from_binary(data);
    let msg: TakeSwapMsg;

    match msg_res {
        Ok(value) => {
            msg = value;
        }
        Err(_err) => {
            let msg_output: TakeSwapMsgOutput = from_binary(data).unwrap();
            msg = TakeSwapMsg {
                order_id: msg_output.order_id.clone(),
                sell_token: msg_output.sell_token.clone(),
                taker_address: msg_output.taker_address.clone(),
                taker_receiving_address: msg_output.taker_receiving_address.clone(),
                timeout_height: Height {
                    revision_number: msg_output.timeout_height.revision_number.parse().unwrap(),
                    revision_height: msg_output.timeout_height.revision_height.parse().unwrap(),
                },
                timeout_timestamp: msg_output.timeout_timestamp.parse().unwrap(),
            }
        }
    }
    msg
}

pub(crate) fn decode_make_swap_msg(data: &Binary) -> MakeSwapMsg {
    let msg_res: Result<MakeSwapMsg, StdError> = from_binary(data);
    let msg: MakeSwapMsg;

    match msg_res {
        Ok(value) => {
            msg = value;
        }
        Err(_err) => {
            let msg_output: MakeSwapMsgOutput = from_binary(data).unwrap();
            msg = MakeSwapMsg {
                source_port: msg_output.source_port.clone(),
                source_channel: msg_output.source_channel.clone(),
                sell_token: msg_output.sell_token.clone(),
                buy_token: msg_output.buy_token.clone(),
                maker_address: msg_output.maker_address.clone(),
                maker_receiving_address: msg_output.maker_receiving_address.clone(),
                desired_taker: msg_output.desired_taker.clone(),
                timeout_height: Height {
                    revision_number: msg_output.timeout_height.revision_number.parse().unwrap(),
                    revision_height: msg_output.timeout_height.revision_height.parse().unwrap(),
                },
                timeout_timestamp: msg_output.timeout_timestamp.parse().unwrap(),
                expiration_timestamp: msg_output.expiration_timestamp.parse().unwrap(),
                take_bids: msg_output.take_bids,
                min_bid_price: None,
            }
        }
    }
    msg
}

pub(crate) fn send_tokens(to: &Addr, amount: Coin) -> StdResult<SubMsg> {
    let msg = BankMsg::Send {
        to_address: to.into(),
        amount: vec![amount],
    };
    Ok(SubMsg::new(msg))
}

/// Calculates taker fees and returns (fee, Value - fee)
pub fn taker_fee(deps: Deps, amount: &Uint128, denom: String) -> (Coin, Coin, Addr) {
    let fee_info = FEE_INFO.load(deps.storage).unwrap();
    let mut fee = (amount * Uint128::from(fee_info.taker_fee)) / Uint128::from(FEE_BASIS_POINT);
    if fee.is_zero() {
        fee = Uint128::from(1u64);
    }
    let treasury_address = deps.api.addr_validate(&fee_info.treasury).unwrap();
    (
        Coin {
            denom: denom.clone(),
            amount: fee,
        },
        Coin {
            denom,
            amount: amount - fee,
        },
        treasury_address,
    )
}

/// Calculates maker fees and returns (fee, Value - fee)
pub fn maker_fee(deps: Deps, amount: &Uint128, denom: String) -> (Coin, Coin, Addr) {
    let fee_info = FEE_INFO.load(deps.storage).unwrap();
    let mut fee = (amount * Uint128::from(fee_info.maker_fee)) / Uint128::from(FEE_BASIS_POINT);
    if fee.is_zero() {
        fee = Uint128::from(1u64);
    }
    let treasury_address = deps.api.addr_validate(&fee_info.treasury).unwrap();
    (
        Coin {
            denom: denom.clone(),
            amount: fee,
        },
        Coin {
            denom,
            amount: amount - fee,
        },
        treasury_address,
    )
}
