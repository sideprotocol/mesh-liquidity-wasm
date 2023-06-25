use cosmwasm_std::{
    from_binary, Addr, BankMsg, Coin, Decimal, IbcAcknowledgement, IbcChannel, IbcOrder, StdResult,
    SubMsg, Uint128, WasmMsg, to_binary,
};
use cw20::Cw20ExecuteMsg;
use sha2::{Digest, Sha256};

use crate::{interchainswap_handler::InterchainSwapPacketAcknowledgement, ContractError, msg::{DepositAsset}};
use hex;

pub fn get_pool_id_with_tokens(tokens: &[Coin]) -> String {
    let mut denoms: Vec<String> = tokens.iter().map(|token| token.denom.clone()).collect();
    denoms.sort();

    // let mut res_vec: Vec<&[u8]> = vec![];
    // for denom in &denoms {
    //     // let denom_data = to_binary(&denom).unwrap();
    //     res_vec.push(to_binary(&denom).unwrap().as_slice());
    // }
    let res = denoms.join("");
    let res_bytes = res.as_bytes();
    let hash = Sha256::digest(&res_bytes);

    let pool_id = format!("pool{}", hex::encode(hash));
    pool_id
}

pub fn uint128_to_f64(value: Uint128) -> f64 {
    let value_str = value.to_string();
    value_str.parse::<f64>().unwrap_or(0.0)
}

pub fn decimal_to_f64(value: Decimal) -> f64 {
    let value_str = value.to_string();
    value_str.parse::<f64>().unwrap_or(0.0)
}

pub fn check_slippage(
    expect: Uint128,
    result: Uint128,
    desire_slippage: u64,
) -> Result<(), ContractError> {
    if desire_slippage > 10000 {
        return Err(ContractError::InvalidSlippage {});
    }

    // Define the slippage tolerance (1% in this example)
    let slippage_tolerance = Uint128::from(desire_slippage);

    // Calculate the absolute difference between the ratios
    let ratio_diff = if expect > result {
        expect - result
    } else {
        result - expect
    };

    // Calculate slippage percentage (slippage = ratio_diff/expect * 100)
    let slippage = ratio_diff * Uint128::from(10000 as u64) / expect;

    // Check if the slippage is within the tolerance
    if slippage >= slippage_tolerance {
        return Err(ContractError::InvalidPairRatio {});
    }

    Ok(())
}

pub fn try_get_ack_error(ack: &IbcAcknowledgement) -> Option<String> {
    let ack: InterchainSwapPacketAcknowledgement =
	// What we can not parse is an ACK fail.
        from_binary(&ack.data).unwrap_or_else(|_| InterchainSwapPacketAcknowledgement::Error(ack.data.to_base64()));
    match ack {
        InterchainSwapPacketAcknowledgement::Error(e) => Some(e),
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

pub fn get_coins_from_deposits(deposits: Vec<DepositAsset>) -> Vec<Coin> {
    let mut tokens = vec![];
    tokens.push(deposits[0].balance.clone());
    tokens.push(deposits[1].balance.clone());
    return tokens;
}

pub(crate) fn send_tokens_coin(to: &Addr, amount: Coin) -> StdResult<Vec<SubMsg>> {
    let msg = BankMsg::Send {
        to_address: to.into(),
        amount: vec![amount],
    };
    Ok(vec![SubMsg::new(msg)])
}

pub fn mint_tokens_cw20(recipient: String, lp_token: String, amount: Uint128) -> StdResult<Vec<SubMsg>> {
    let msg = Cw20ExecuteMsg::Mint {
        recipient: recipient.into(),
        amount: amount,
    };
    let exec = WasmMsg::Execute {
        contract_addr: lp_token.into(),
        msg: to_binary(&msg)?,
        funds: vec![],
    };
    Ok(vec![SubMsg::new(exec)])
}

// pub(crate) fn decode_create_pool_msg(data: &Binary) -> MsgMakePoolRequest {
//     let msg_res: Result<MsgMakePoolRequest, StdError> = from_binary(data);
//     let msg: MsgMakePoolRequest;

//     match msg_res {
//         Ok(value) => {
//             msg = value.clone();
//         }
//         Err(_err) => {
//             // TODO:handle error
//             // Why do we need MSgOUtput ? does it not unwrap string
//             // let msg_output: MsgMakePoolRequest = from_binary(data).unwrap();
//             // msg = MsgMakePoolRequest {
//             //     source_port: msg_output.source_port.clone(),
//             //     source_channel: msg_output.source_channel.clone(),
//             //     sender: msg_output.sender,
//             //     tokens: msg_output.tokens,
//             //     decimals: msg_output.decimals,
//             //     weights: msg_output.weights,
//             // }
//         }
//     }
//     msg
// }
