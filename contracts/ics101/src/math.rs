use crate::{approx_pow::calculate_pow, types::WeightedAsset};
use cosmwasm_std::{Decimal, StdError, StdResult, Uint128};
use crate::utils::adjust_precision;

// Referenced from Balancer Weighted pool implementation by  Osmosis here - https://github.com/osmosis-labs/osmosis/blob/47a2366c5eeee474de9e1cb4777fab0ccfbb9592/x/gamm/pool-models/balancer/amm.go#L94
// solveConstantFunctionInvariant solves the constant function of an AMM
// https://github.com/dexter-zone/dexter_core/tree/main/contracts/pools/weighted_pool
// that determines the relationship between the differences of two sides
// of assets inside the pool.
// --------------------------
// For fixed balanceXBefore, balanceXAfter, weightX, balanceY, weightY,
// we could deduce the balanceYDelta, calculated by:
// balanceYDelta = balanceY * (1 - (balanceXBefore/balanceXAfter)^(weightX/weightY))
// balanceYDelta is positive when the balance liquidity decreases.
// balanceYDelta is negative when the balance liquidity increases.
pub fn solve_constant_function_invariant(
    token_balance_fixed_before: Decimal,
    token_balance_fixed_after: Decimal,
    token_weight_fixed: Decimal,
    token_balance_unknown_before: Decimal,
    token_weight_unknown: Decimal,
) -> StdResult<Decimal> {
    // weight_ratio = (weightX/weightY)
    let weight_ratio = token_weight_fixed
        .checked_div(token_weight_unknown)
        .map_err(|e| StdError::generic_err(e.to_string()))?;

    // y = balanceXBefore/balanceXAfter
    let y = token_balance_fixed_before
        .checked_div(token_balance_fixed_after)
        .map_err(|e| StdError::generic_err(e.to_string()))?;

    // amount_y = balanceY * (1 - (y ^ weight_ratio))
    let y_to_weight_ratio = calculate_pow(y, weight_ratio, None)?;
    // Decimal is an unsigned so always return abs value
    let paranthetical = if y_to_weight_ratio <= Decimal::one() {
        Decimal::one().checked_sub(y_to_weight_ratio)?
    } else {
        y_to_weight_ratio.checked_sub(Decimal::one())?
    };

    let amount_y = token_balance_unknown_before.checked_mul(paranthetical)?;
    return Ok(amount_y);
}

/// ## Description - Inspired from Osmosis implementaton here - https://github.com/osmosis-labs/osmosis/blob/main/x/gamm/pool-models/balancer/amm.go#L116
/// Calculates the amount of LP shares to be minted for Single asset joins.
pub fn calc_minted_shares_given_single_asset_in(
    token_amount_in: Uint128,
    in_precision: u32,
    asset_weight_and_balance: &WeightedAsset,
    total_shares: Uint128,
    swap_fee_rate: Decimal,
) -> StdResult<(Uint128, Uint128)> {
    // deduct swapfee on the in asset.
    // We don't charge swap fee on the token amount that we imagine as unswapped (the normalized weight).
    // So, effective_swapfee = swapfee * (1 - normalized_token_weight)
    let fee_ratio = fee_ratio(asset_weight_and_balance.weight, swap_fee_rate);
    let token_amount_in_after_fee = token_amount_in * fee_ratio;
    let fee_charged = token_amount_in.checked_sub(token_amount_in_after_fee)?;

    let in_decimal = Decimal::from_atomics(token_amount_in_after_fee, in_precision).unwrap();
    let balance_decimal =
        Decimal::from_atomics(asset_weight_and_balance.asset.amount, in_precision).unwrap();

    // To figure out the number of shares we add, first notice that we can treat
    // the number of shares as linearly related to the `k` value function. This is due to the normalization.
    // e.g, if x^.5 y^.5 = k, then we `n` x the liquidity to `(nx)^.5 (ny)^.5 = nk = k'`
    // ---------
    // We generalize this linear relation to do the liquidity add for the not-all-asset case.
    // Suppose we increase the supply of x by x', so we want to solve for `k'/k`.
    // This is `(x + x')^{weight} * old_terms / (x^{weight} * old_terms) = (x + x')^{weight} / (x^{weight})`
    // The number of new shares we need to make is then `old_shares * ((k'/k) - 1)`
    let pool_amount_out = solve_constant_function_invariant(
        balance_decimal + in_decimal,
        balance_decimal,
        asset_weight_and_balance.weight,
        Decimal::from_atomics(total_shares, Decimal::DECIMAL_PLACES).unwrap(),
        Decimal::one(),
    )?;
    let pool_amount_out_adj = adjust_precision(
        pool_amount_out.atomics(),
        pool_amount_out.decimal_places() as u8,
        Decimal::DECIMAL_PLACES as u8,
    )?;

    return Ok((pool_amount_out_adj, fee_charged));
}

// feeRatio returns the fee ratio that is defined as follows:
// 1 - ((1 - normalizedTokenWeightOut) * swapFee)
fn fee_ratio(normalized_weight: Decimal, swap_fee: Decimal) -> Decimal {
    return Decimal::one() - ((Decimal::one() - normalized_weight) * swap_fee);
}

// ## Description
// Calculates the weight of an asset as % of the total weight share. Returns a decimal.
// ## Params
// * **weight** is the weight of the asset.
// * **total_weight** is the total weight of all assets.
// pub fn get_normalized_weight(weight: Uint128, total_weight: Uint128) -> Decimal {
//     Decimal::from_ratio(weight, total_weight)
// }

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    #[test]
    fn test_solve_constant_function_invariant() {
        // Define some example inputs for the function
        let token_balance_fixed_before = Decimal::from_str("500000000000").unwrap();
        let token_balance_fixed_after = Decimal::from_str( "530000000000").unwrap();
        let token_weight_fixed = Decimal::from_str("0.5").unwrap();
        let token_balance_unknown_before = Decimal::from_str("500000000000").unwrap();
        let token_weight_unknown = Decimal::from_str("0.5").unwrap();

        // Call the function with the example inputs
        let result = solve_constant_function_invariant(
            token_balance_fixed_before,
            token_balance_fixed_after,
            token_weight_fixed,
            token_balance_unknown_before,
            token_weight_unknown,
        );

        // Assert the result is as expected
        assert!(result.is_ok());
        let amount_y = result.unwrap();
        let res = adjust_precision(amount_y.to_uint_floor(), 12, 6).unwrap();
        assert_eq!(res, Uint128::from(28301u128));
    }
}