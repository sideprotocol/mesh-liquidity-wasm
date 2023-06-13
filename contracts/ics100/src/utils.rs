use cosmwasm_std::{
    from_binary, to_binary, Addr, BankMsg, Binary, Coin, IbcAcknowledgement, IbcChannel, IbcOrder,
    StdError, StdResult, SubMsg,
};
use sha2::{Digest, Sha256};

use crate::{
    atomic_swap_handler::AtomicSwapPacketAcknowledgement,
    msg::{Height, HeightOutput, MakeSwapMsg, MakeSwapMsgOutput, TakeSwapMsg, TakeSwapMsgOutput},
    ContractError,
};

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
            msg = value.clone();
        }
        Err(_err) => {
            let msg_output: TakeSwapMsgOutput = from_binary(data).unwrap();
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
    msg
}

pub(crate) fn decode_make_swap_msg(data: &Binary) -> MakeSwapMsg {
    let msg_res: Result<MakeSwapMsg, StdError> = from_binary(data);
    let msg: MakeSwapMsg;

    match msg_res {
        Ok(value) => {
            msg = value.clone();
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
                create_timestamp: msg_output.create_timestamp.parse().unwrap(),
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
                expiration_timestamp: msg_output.expiration_timestamp.parse().unwrap(),
            }
        }
    }
    msg
}

pub(crate) fn send_tokens(to: &Addr, amount: Coin) -> StdResult<Vec<SubMsg>> {
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


// Add function to transfer cw20 tokens