use std::str::FromStr;

use cosmwasm_std::{StdError, StdResult, Uint128};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

/** Inflation rate, and other fun things are in the form 0.xxxxx.
 * To use we remove the leading '0.'
 * and cut all but the the first 4 digits
 */
#[allow(dead_code)]
pub fn dec_to_uint(dec: String) -> StdResult<u128> {
    let tokens: Vec<&str> = dec.split('.').collect();

    if tokens.len() < 2 {
        return u128::from_str(&dec).map_err(|_| StdError::generic_err("failed to parse number"));
    }

    u128::from_str(&dec).map_err(|_| StdError::generic_err("failed to parse number"))
}

/**
 * Calculates how much your withdrawn tokens are worth in SIDE
 */
pub fn calc_withdraw(amount: Uint128, exchange_rate: Decimal) -> StdResult<u128> {
    let normalized_amount = Decimal::from(amount.u128() as u64);

    let raw_amount = normalized_amount
        .checked_mul(exchange_rate)
        .unwrap_or(Decimal::from(0u128));

    let coins_to_withdraw = raw_amount.to_u128().unwrap();

    Ok(coins_to_withdraw)
}

/**
 * Calculates threshold amount from reward amount
 * and count of validators.
 *
 * Returns amount of SIDE tokens as threshold value.
 */
pub fn calc_threshold(amount: u128, val_count: usize) -> u128 {
    amount.checked_div(val_count as u128).unwrap_or(0)
}
