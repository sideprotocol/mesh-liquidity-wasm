use cosmwasm_std::{Addr, BankMsg, Coin, Deps, StdError, StdResult, SubMsg, Uint128};
use sha2::{Digest, Sha256};

use crate::state::FEE_INFO;

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
