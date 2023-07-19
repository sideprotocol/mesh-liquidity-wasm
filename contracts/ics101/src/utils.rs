use std::{ops::{Div}, vec};

use cosmwasm_std::{
    from_binary, Addr, BankMsg, Coin, IbcAcknowledgement, IbcChannel, IbcOrder, StdResult,
    SubMsg, Uint128, WasmMsg, to_binary, Decimal, Decimal256, StdError,
};
use cw20::Cw20ExecuteMsg;
use sha2::{Digest, Sha256};

use crate::{interchainswap_handler::InterchainSwapPacketAcknowledgement, ContractError, msg::{DepositAsset}, market::PoolAsset};
use hex;

pub const MULTIPLIER: u128 = 1e18 as u128;
pub const MAXIMUM_SLIPPAGE: u64 = 10000;
pub const INSTANTIATE_TOKEN_REPLY_ID: u64 = 2000;

pub fn get_pool_id_with_tokens(tokens: &[Coin], source: String, destination: String) -> String {
    let mut denoms: Vec<String> = tokens.iter().map(|token| token.denom.clone()).collect();
    denoms.sort();
    let mut chan = vec![];
    chan.push(source);
    chan.push(destination);
    let connection = get_connection_id(chan);

    let mut res = denoms.join("");
    res = res + &connection;
    let res_bytes = res.as_bytes();
    let hash = Sha256::digest(&res_bytes);

    let pool_id = format!("pool{}", hex::encode(hash));
    pool_id
}

pub fn get_connection_id(mut chain_ids: Vec<String>) -> String {
    chain_ids.sort();

    let res = chain_ids.join("/");
    return res;
}

/// ## Description
/// Return a value using a newly specified precision.
/// ## Params
/// * **value** is an object of type [`Uint128`]. This is the value that will have its precision adjusted.
/// * **current_precision** is an object of type [`u8`]. This is the `value`'s current precision
/// * **new_precision** is an object of type [`u8`]. This is the new precision to use when returning the `value`.
pub fn adjust_precision(
    value: Uint128,
    current_precision: u8,
    new_precision: u8,
) -> StdResult<Uint128> {
    Ok(match current_precision.cmp(&new_precision) {
        std::cmp::Ordering::Equal => value,
        std::cmp::Ordering::Less => value.checked_mul(Uint128::new(
            10_u128.pow((new_precision - current_precision) as u32),
        ))?,
        std::cmp::Ordering::Greater => value.checked_div(Uint128::new(
            10_u128.pow((current_precision - new_precision) as u32),
        ))?,
    })
}

/// ## Description
/// Converts [`Decimal`] to [`Decimal256`].
pub fn decimal2decimal256(dec_value: Decimal) -> StdResult<Decimal256> {
    Decimal256::from_atomics(dec_value.atomics(), dec_value.decimal_places()).map_err(|_| {
        StdError::generic_err(format!(
            "Failed to convert Decimal {} to Decimal256",
            dec_value
        ))
    })
}

pub fn get_precision(assets: Vec<PoolAsset>, token: Coin) -> u32 {
    for asset in assets {
        if asset.balance.denom == token.denom {
            return asset.decimal
        }
    }
    // we already check if asset is present in pool asset vector
    // this code is unreachable
    return 1;
}

pub fn check_slippage(
    source_amount: Uint128,
    destination_amount: Uint128,
    source_balance: Uint128,
    destination_balance: Uint128,
    desire_slippage: u64,
) -> Result<(), ContractError> {
    // Check the ratio of local amount and remote amount
    let expect = source_amount
        .div(destination_amount);
    let result = source_balance
        .div(Uint128::from(destination_balance));

    if desire_slippage > MAXIMUM_SLIPPAGE {
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

pub const ICS101_VERSION: &str = "ics101-1";
pub const ICS101_ORDERING: IbcOrder = IbcOrder::Unordered;

pub(crate) fn enforce_order_and_version(
    channel: &IbcChannel,
    counterparty_version: Option<&str>,
) -> Result<(), ContractError> {
    if channel.version != ICS101_VERSION {
        return Err(ContractError::InvalidIbcVersion {
            version: channel.version.clone(),
        });
    }
    if let Some(version) = counterparty_version {
        if version != ICS101_VERSION {
            return Err(ContractError::InvalidIbcVersion {
                version: version.to_string(),
            });
        }
    }
    if channel.order != ICS101_ORDERING {
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

pub fn burn_tokens_cw20(lp_token: String, amount: Uint128) -> StdResult<SubMsg> {
    let msg = Cw20ExecuteMsg::Burn {
        amount: amount,
    };
    let exec = WasmMsg::Execute {
        contract_addr: lp_token.into(),
        msg: to_binary(&msg)?,
        funds: vec![],
    };
    Ok(SubMsg::new(exec))
}

pub fn send_tokens_cw20(recipient: String, lp_token: String, amount: Uint128) -> StdResult<Vec<SubMsg>> {
    let msg = Cw20ExecuteMsg::Transfer {
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

/// Checks the validity of the token name
pub fn is_valid_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.len() < 3 || bytes.len() > 50 {
        return false;
    }
    true
}

/// Checks the validity of the token symbol
pub fn is_valid_symbol(symbol: &str, max_length: Option<usize>) -> bool {
    let max_length = max_length.unwrap_or(12);
    let bytes = symbol.as_bytes();
    if bytes.len() < 3 || bytes.len() > max_length {
        return false;
    }
    for byte in bytes.iter() {
        if (*byte != 45)
            && (*byte < 47 || *byte > 57)
            && (*byte < 65 || *byte > 90)
            && (*byte < 97 || *byte > 122)
        {
            return false;
        }
    }
    true
}

