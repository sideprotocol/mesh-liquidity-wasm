use std::{
   vec, str::FromStr,
};

use cosmwasm_std::{Coin, Decimal, StdError, StdResult, Uint128, Decimal256, Uint256};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{math::{calc_minted_shares_given_single_asset_in, solve_constant_function_invariant}, types::WeightedAsset, utils::{decimal2decimal256, adjust_precision} };

pub const FEE_PRECISION: u16 = 10000;
pub const FIXED_PRECISION: u8 = 12;
/// Number of LP tokens to mint when liquidity is provided for the first time to the pool.
/// This does not include the token decimals.
// const INIT_LP_TOKENS: u128 = 100;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub enum PoolSide {
    SOURCE = 0,
    DESTINATION = 1,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub enum PoolStatus {
    #[serde(rename = "INITIALIZED")]
    Initialized = 0,
    #[serde(rename = "ACTIVE")]
    Active = 1,
    #[serde(rename = "CANCELLED")]
    Cancelled = 2,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct PoolAsset {
    pub side: PoolSide,
    pub balance: Coin,
    pub weight: u32,
    pub decimal: u32,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InterchainLiquidityPool {
    pub assets: Vec<PoolAsset>,
    pub counter_party_channel: String,
    pub counter_party_port: String,
    pub destination_creator: String,
    pub destination_chain_id: String,
    pub id: String,
    pub source_chain_id: String,
    pub source_creator: String,
    pub status: PoolStatus,
    pub supply: Coin,
    pub swap_fee: u32,
    pub pool_price: u64
}

impl InterchainLiquidityPool {
    pub fn find_asset_by_denom(&self, denom: &str) -> StdResult<PoolAsset> {
        for asset in &self.assets {
            if asset.balance.denom == denom {
                return Ok(asset.clone());
            }
        }
        Err(StdError::generic_err("Denom not found in pool"))
    }

    pub fn find_asset_by_side(&self, side: PoolSide) -> StdResult<PoolAsset> {
        for asset in &self.assets {
            if asset.side == side {
                return Ok(asset.clone())
            }
        }
        Err(StdError::generic_err("Asset side not found in pool"))
    }

    pub fn add_asset(&mut self, token: Coin) -> StdResult<Coin> {
        let mut indx = 0;
        let mut found = false;
        for (idx, asset) in self.assets.iter().enumerate() {
            if asset.balance.denom == token.denom {
                indx = idx;
                found = true;
            }
        }

        if !found {
            return Err(StdError::generic_err("Denom not found in pool"));
        }
        self.assets[indx].balance.amount += token.amount;
        Ok(token)
    }

    pub fn add_supply(&mut self, token: Coin) -> StdResult<Coin> {
        if self.supply.denom == token.denom {
            self.supply.amount += token.amount;
            Ok(token)
        } else {
            Err(StdError::generic_err("Denom not found"))
        }
    }

    pub fn subtract_asset(&mut self, token: Coin) -> StdResult<Coin> {
        let mut indx = 0;
        let mut found = false;
        for (idx, asset) in self.assets.iter().enumerate() {
            if asset.balance.denom == token.denom {
                indx = idx;
                found = true;
            }
        }

        if !found {
            return Err(StdError::generic_err("Denom not found in pool"));
        }
        self.assets[indx].balance.amount -= token.amount;
        Ok(token)
    }

    pub fn subtract_supply(&mut self, token: Coin) -> StdResult<Coin> {
        if self.supply.denom == token.denom {
            self.supply.amount -= token.amount;
            Ok(token)
        } else {
            Err(StdError::generic_err("Denom not found"))
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InterchainMarketMaker {
    pub pool: InterchainLiquidityPool,
    pub fee_rate: u32,
}

impl InterchainMarketMaker {
    pub fn new(pool_data: &InterchainLiquidityPool, fee_rate: u32) -> Self {
        InterchainMarketMaker {
            pool: pool_data.clone(),
            fee_rate,
        }
    }
    
    /// Calculate the amount of LP tokens that should be minted for single asset deposit.
    /// Returns the amount of LP tokens to be minted
    pub fn deposit_single_asset(&self, token: &Coin) -> StdResult<Coin> {
        let asset = self
            .pool
            .assets
            .iter()
            .find(|a| a.balance.denom == token.denom)
            .ok_or_else(|| StdError::generic_err("Asset not found"))?;

        let issue_amount;
        let _fee_charged;

        if self.pool.status != PoolStatus::Active {
            return Err(StdError::generic_err("Pool is not active!"));
        } else {
            let pool_asset_weighted = &WeightedAsset {
                asset: token.clone(),
                weight: Decimal::from_ratio(asset.weight, Uint128::from(100u64))
            };

            // Asset weights already normalized
            (issue_amount, _fee_charged) = calc_minted_shares_given_single_asset_in(
                token.amount,
                asset.decimal.into(),
                pool_asset_weighted,
                self.pool.supply.amount,
                Decimal::from_ratio(self.fee_rate, FEE_PRECISION),
            )?;
        }

        let output_token = Coin {
            amount: issue_amount,
            denom: self.pool.clone().supply.denom,
        };
        Ok(output_token)
    }

    // P_issued = P_supply * Wt * Dt/Bt
    pub fn deposit_multi_asset(&self, tokens: &[Coin]) -> StdResult<Vec<Coin>> {
        let mut out_tokens = vec![];
        for token in tokens {
            let asset = self.pool.clone().find_asset_by_denom(&token.denom)?;
            let mut total_asset_amount = Uint128::from(0u128);
            let mut issue_amount;
            if self.pool.status == PoolStatus::Initialized && self.pool.supply.amount.is_zero() {
                for asset in &self.pool.assets {
                    let dec_asset_amount = adjust_precision(asset.balance.amount, asset.decimal.try_into().unwrap(), 18)?;
                    total_asset_amount = total_asset_amount + dec_asset_amount;
                }
                let mult_amount = total_asset_amount.checked_mul(asset.weight.into())?;
                issue_amount = Decimal::from_ratio(mult_amount, Uint128::from(100u128));
            } else {
                let ratio = Decimal::from_ratio(token.amount, asset.balance.amount);
                issue_amount = Decimal::from_ratio(self.pool.supply.amount, Uint128::from(100u128));
                issue_amount = issue_amount.checked_mul(ratio)?;
                issue_amount = issue_amount.checked_mul(Decimal::from_str(&asset.weight.to_string())?)?;
            }

            let output_token = Coin {
                denom: self.pool.supply.denom.clone(),
                amount: issue_amount.to_uint_ceil()
            };
            out_tokens.push(output_token)
        }
        return Ok(out_tokens)
    }

    pub fn multi_asset_withdraw(&self, redeem: Coin) -> StdResult<Vec<Coin>> {
        let total_share = self.pool.supply.amount.clone();

        // % of share to be burnt from the pool
        let share_out_ratio = Decimal::from_ratio(redeem.amount, total_share);
    
        // Vector of assets to be transferred to the user from the Vault contract
        let mut refund_assets: Vec<Coin> = vec![];
        for asset in &self.pool.assets {
            let asset_out = asset.balance.amount * share_out_ratio;
            // Return a `Failure` response if the calculation of the amount of tokens to be burnt from the pool is not valid
            if asset_out > asset.balance.amount {
                return Err(StdError::generic_err("Invalid asset out"));
            }
            // Add the asset to the vector of assets to be transferred to the user from the Vault contract
            refund_assets.push(Coin {
                denom: asset.balance.denom.clone(),
                amount: asset_out,
            });
        }

        Ok(refund_assets)
    }

    // --------x--------x--------x--------x--------x--------x--------x--------x---------
    // --------x--------x SWAP :: Offer and Ask amount computations  x--------x---------
    // --------x--------x--------x--------x--------x--------x--------x--------x---------

    /// ## Description
    ///  Returns the result of a swap, if erros then returns [`ContractError`].
    ///
    /// ## Params
    /// * **config** is an object of type [`Config`].
    /// * **offer_asset** is an object of type [`Asset`]. This is the asset that is being offered.
    /// * **offer_pool** is an object of type [`DecimalAsset`]. This is the pool of offered asset.
    /// * **ask_pool** is an object of type [`DecimalAsset`]. This is the asked asset.
    /// * **pools** is an array of [`DecimalAsset`] type items. These are the assets available in the pool.
    pub fn compute_swap(&self, amount_in: Coin, denom_out: &str) -> StdResult<Coin> {
        let asset_in = self.pool.clone().find_asset_by_denom(&amount_in.denom)?;
        let asset_out = self.pool.clone().find_asset_by_denom(denom_out)?;

        let token_precision = asset_out.decimal as u8;

        let pool_post_swap_in_balance = asset_in.balance.amount + self.minus_fees(amount_in.amount).to_uint_floor();

        //         /**********************************************************************************************
        //         // outGivenIn                                                                                //
        //         // aO = amountOut                                                                            //
        //         // bO = balanceOut                                                                           //
        //         // bI = balanceIn              /      /            bI             \    (wI / wO) \           //
        //         // aI = amountIn    aO = bO * |  1 - | --------------------------  | ^            |          //
        //         // wI = weightIn               \      \       ( bI + aI )         /              /           //
        //         // wO = weightOut                                                                            //
        //         **********************************************************************************************/
        // delta balanceOut is positive(tokens inside the pool decreases)

        let token_balance_fixed_before = 
            adjust_precision(asset_in.balance.amount, asset_in.decimal.try_into().unwrap(), FIXED_PRECISION)?;
        let token_balance_fixed_after = 
            adjust_precision(pool_post_swap_in_balance, asset_in.decimal.try_into().unwrap(), FIXED_PRECISION)?;
        let token_balance_unknown_before = 
            adjust_precision(asset_out.balance.amount, asset_out.decimal.try_into().unwrap(), FIXED_PRECISION)?;

        let return_amount = solve_constant_function_invariant(
            Decimal::from_str(&token_balance_fixed_before.to_string())?,
            Decimal::from_str(&token_balance_fixed_after.to_string())?,
            Decimal::from_ratio(asset_in.weight, Uint128::from(100u64)),
            Decimal::from_str(&token_balance_unknown_before.to_string())?,
            Decimal::from_ratio(asset_out.weight, Uint128::from(100u64)),
        )?;
    
        // adjust return amount to correct precision
        let return_amount = adjust_precision(
            return_amount.to_uint_floor(),
            FIXED_PRECISION,
            token_precision,
        )?;

        Ok(Coin {
            amount: return_amount,
            denom: denom_out.to_string(),
        })
    }

    pub fn compute_offer_amount(&self, amount_in: Coin, amount_out: Coin) -> StdResult<Coin> {
        let asset_in = self.pool.clone().find_asset_by_denom(&amount_in.denom)?;
        let asset_out = self.pool.clone().find_asset_by_denom(&amount_out.denom)?;

        // get ask asset precisison
        let token_precision = asset_in.decimal as u8;
        let one_minus_commission = Decimal256::one()
            - decimal2decimal256(Decimal::from_ratio(self.fee_rate, FEE_PRECISION))?;
        let inv_one_minus_commission = Decimal256::one() / one_minus_commission;

        let ask_asset_amount = &amount_out.amount.clone();
        // Ask pool balance after swap
        let pool_post_swap_out_balance = asset_out.balance.amount - ask_asset_amount;

        //         /**********************************************************************************************
        //         // inGivenOut                                                                                //
        //         // aO = amountOut                                                                            //
        //         // bO = balanceOut                                                                           //
        //         // bI = balanceIn              /  /            bO             \    (wO / wI)      \          //
        //         // aI = amountIn    aI = bI * |  | --------------------------  | ^            - 1  |         //
        //         // wI = weightIn               \  \       ( bO - aO )         /                   /          //
        //         // wO = weightOut                                                                            //
        //         **********************************************************************************************/
        // delta balanceOut is positive(tokens inside the pool decreases)

        let token_balance_fixed_before = 
            adjust_precision(asset_out.balance.amount, asset_out.decimal.try_into().unwrap(), FIXED_PRECISION)?;
        let token_balance_fixed_after = 
            adjust_precision(pool_post_swap_out_balance, asset_out.decimal.try_into().unwrap(), FIXED_PRECISION)?;
        let token_balance_unknown_before = 
            adjust_precision(asset_in.balance.amount, asset_in.decimal.try_into().unwrap(), FIXED_PRECISION)?;

        let real_offer = solve_constant_function_invariant(
        Decimal::from_str(&token_balance_fixed_before.to_string())?,
        Decimal::from_str(&token_balance_fixed_after.to_string())?,
        Decimal::from_ratio(asset_out.weight, Uint128::from(100u64)),
        Decimal::from_str(&token_balance_unknown_before.to_string())?,
        Decimal::from_ratio(asset_in.weight, Uint128::from(100u64)),
        )?; 
        // adjust return amount to correct precision
        let real_offer = adjust_precision(
        real_offer.to_uint_floor(),
        FIXED_PRECISION,
        token_precision,
        )?;
       
        let offer_amount_including_fee = (Uint256::from(real_offer) * inv_one_minus_commission).try_into()?;
        let _total_fee = offer_amount_including_fee - real_offer;

        Ok(Coin {
            amount: offer_amount_including_fee,
            denom: amount_in.denom,
        })
    }

    pub fn minus_fees(&self, amount: Uint128) -> Decimal {
        let amount_dec = Decimal::from_ratio(amount.u128(), Uint128::one());
        let fee_rate_dec = Decimal::from_ratio(self.fee_rate, Uint128::new(10000));
        let fees = amount_dec * fee_rate_dec;
        let amount_minus_fees = amount_dec - fees;
        amount_minus_fees
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct MarketFeeUpdateProposal {
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "pool_id")]
    pub pool_id: String,
    #[serde(rename = "fee_rate")]
    pub fee_rate: u32,
}
