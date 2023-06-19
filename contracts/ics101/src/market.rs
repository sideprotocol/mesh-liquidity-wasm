use std::{
    f64::EPSILON,
    ops::{Add, Div, Mul, Sub},
};

use cosmwasm_std::{Coin, Decimal, StdError, StdResult, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::utils::{decimal_to_f64, uint128_to_f64};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub enum PoolSide {
    SOURCE = 0,
    DESTINATION = 1,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub enum PoolStatus {
    #[serde(rename = "POOL_STATUS_INITIALIZED")]
    PoolStatusInitialized = 0,
    #[serde(rename = "POOL_STATUS_ACTIVE")]
    PoolStatusActive = 1,
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
    pub pool_id: String,
    pub source_creator: String,
    pub destination_creator: String,
    pub assets: Vec<PoolAsset>,
    pub swap_fee: u32,
    pub supply: Coin,
    pub status: PoolStatus,
    pub pool_price: f32,
    pub originating_chain_id: String,
    pub counter_party_port: String,
    pub counter_party_channel: String,
}

impl InterchainLiquidityPool {
    pub fn find_asset_by_denom(self, denom: &str) -> StdResult<PoolAsset> {
        for asset in self.assets {
            if asset.balance.denom == denom {
                return Ok(asset);
            }
        }
        Err(StdError::generic_err("Denom not found in pool"))
    }

    pub fn find_asset_by_side(self, side: PoolSide) -> StdResult<PoolAsset> {
        for asset in self.assets {
            if asset.side == side {
                return Ok(asset)
            }
        }
        Err(StdError::generic_err("Asset side not found in pool"))
    }

    pub fn add_asset(mut self, token: Coin) -> StdResult<Coin> {
        for mut asset in self.assets {
            if asset.balance.denom == token.denom {
                asset.balance.amount += token.amount;
            }
        }
        Err(StdError::generic_err("Denom not found in pool"))
    }

    pub fn add_supply(mut self, token: Coin) -> StdResult<Coin> {
        if self.supply.denom == token.denom {
            self.supply.amount += token.amount
        }
        Err(StdError::generic_err("Denom not found"))
    }

    pub fn subtract_asset(mut self, token: Coin) -> StdResult<Coin> {
        for mut asset in self.assets {
            if asset.balance.denom == token.denom {
                asset.balance.amount -= token.amount;
            }
        }
        Err(StdError::generic_err("Denom not found in pool"))
    }

    pub fn subtract_supply(mut self, token: Coin) -> StdResult<Coin> {
        if self.supply.denom == token.denom {
            self.supply.amount -= token.amount
        }
        Err(StdError::generic_err("Denom not found"))
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InterchainMarketMaker {
    pub pool_id: String,
    pub pool: InterchainLiquidityPool,
    pub fee_rate: u32,
}

impl InterchainMarketMaker {
    pub fn new(pool_data: &InterchainLiquidityPool, fee_rate: u32) -> Self {
        InterchainMarketMaker {
            pool_id: pool_data.clone().pool_id,
            pool: pool_data.clone(),
            fee_rate,
        }
    }

    pub fn invariant_with_input(&self, token_in: &Coin) -> f64 {
        let mut v = 1.0;
        let mut total_balance = Decimal::zero();

        for pool in self.pool.clone().assets {
            total_balance = total_balance + &Decimal::new(pool.balance.amount.clone());

            if pool.balance.denom == token_in.denom {
                total_balance = total_balance + &Decimal::new(token_in.amount.clone());
            }
        }

        for pool in self.pool.clone().assets {
            let w = (pool.weight as f64) / 100.0;
            let balance = if token_in.denom != pool.balance.denom {
                Decimal::new(pool.balance.amount.clone()).div(&total_balance)
            } else {
                Decimal::new(pool.balance.amount.clone().add(token_in.amount.clone()))
                    .div(&total_balance)
            };
            v *= decimal_to_f64(balance.clone()).powf(w);
        }

        v
    }

    pub fn deposit_single_asset(&self, token: &Coin) -> StdResult<Coin> {
        let asset = self
            .pool
            .assets
            .iter()
            .find(|a| a.balance.denom == token.denom)
            .ok_or_else(|| StdError::generic_err("Asset not found"))?;

        let issue_amount: Uint128;

        if self.pool.status != PoolStatus::PoolStatusActive {
            // throw error
        } else {
            let weight = (asset.weight as f64) / 100.0;

            let ratio = 1.0
                + (uint128_to_f64(token.amount.clone()))
                    / (uint128_to_f64(asset.balance.amount.clone()));

            let factor = (ratio.powf(weight) - 1.0) * 1e18 as f64;

            issue_amount = self
                .pool
                .supply
                .amount
                .mul(Uint128::from(factor as u64))
                .div(Uint128::from(1e18 as u64));

            // Check if we need this
            let estimated_amount = self.pool.supply.amount + issue_amount.clone();

            let estimated_lp_price =
                self.invariant_with_input(token) / uint128_to_f64(estimated_amount);
            let pool_price = self.pool.pool_price as f64;
            if (estimated_lp_price - pool_price).abs() / pool_price > EPSILON {
                return Err(StdError::generic_err("Not allowed amount"));
            }
        }

        let output_token = Coin {
            amount: issue_amount,
            denom: self.pool.clone().supply.denom,
        };
        Ok(output_token)
    }

    pub fn deposit_multi_asset(&self, tokens: &[Coin]) -> StdResult<Vec<Coin>> {
        let mut out_tokens: Vec<Coin> = Vec::new();

        for token in tokens {
            let asset = self.pool.clone().find_asset_by_denom(&token.denom)?;

            let issue_amount;

            if self.pool.status == PoolStatus::PoolStatusInitialized {
                let mut total_initial_lp_amount = Uint128::zero();
                for asset in self.pool.clone().assets {
                    total_initial_lp_amount = total_initial_lp_amount.add(asset.balance.amount);
                }
                issue_amount = total_initial_lp_amount;
            } else {
                let ratio = (token.amount.u128() as f64 / asset.balance.amount.u128() as f64
                    * 1e18 as f64) as u128;
                issue_amount =
                    self.pool.supply.amount * Uint128::from(ratio) / Uint128::from(1e18 as u64);
            }

            let output_token = Coin {
                amount: issue_amount,
                denom: self.pool.supply.denom.clone(),
            };
            out_tokens.push(output_token);
        }

        Ok(out_tokens)
    }

    pub fn single_withdraw(&self, redeem: Coin, denom_out: &str) -> StdResult<Coin> {
        let asset = self.pool.clone().find_asset_by_denom(denom_out)?;

        if redeem.amount > self.pool.supply.amount {
            return Err(StdError::generic_err("Amount exceeds balance"));
        }

        if redeem.denom != self.pool.supply.denom {
            return Err(StdError::generic_err("Invalid token pair"));
        }

        let ratio = self
            .pool
            .supply
            .amount
            .sub(redeem.amount)
            .mul(Uint128::from(1e18 as u128))
            .div(self.pool.supply.amount);
        let ratio_float = uint128_to_f64(ratio) / 1e18 as f64;

        let exponent = 1.0 / asset.weight as f64;
        let factor = (1.0 - f64::powf(ratio_float, exponent)) * 1e18 as f64;
        let amount_out = asset
            .balance
            .amount
            .mul(Uint128::new(factor as u128))
            .div(Uint128::from(1e18 as u128));

        Ok(Coin {
            amount: amount_out,
            denom: denom_out.to_string(),
        })
    }

    pub fn multi_asset_withdraw(&self, redeem: Coin, denom_out: &str) -> StdResult<Coin> {
        let asset = self.pool.clone().find_asset_by_denom(denom_out)?;

        let out = asset
            .balance
            .amount
            .mul(redeem.amount)
            .div(self.pool.supply.amount);

        Ok(Coin {
            denom: denom_out.to_string(),
            amount: out,
        })
    }

    pub fn left_swap(&self, amount_in: Coin, denom_out: &str) -> StdResult<Coin> {
        let asset_in = self.pool.clone().find_asset_by_denom(&amount_in.denom)?;

        let asset_out = self.pool.clone().find_asset_by_denom(denom_out)?;

        let balance_out = asset_out.balance.amount;

        let balance_in = asset_in.balance.amount;

        let weight_in = Decimal::from_ratio(asset_in.weight, Uint128::new(100));

        let weight_out = Decimal::from_ratio(asset_out.weight, Uint128::new(100));

        let amount = self.minus_fees(amount_in.amount);

        let balance_in_plus_amount = balance_in + amount.to_uint_floor();
        let ratio = balance_in / balance_in_plus_amount;
        let one_minus_ratio = Decimal::one().sub(Decimal::new(ratio));
        let power = weight_in / weight_out;
        let factor = decimal_to_f64(one_minus_ratio).powf(decimal_to_f64(power)) * 1e18;
        let amount_out = balance_out
            * Decimal::from_ratio(Uint128::from(factor as u128), Uint128::from(1e18 as u64));

        Ok(Coin {
            amount: amount_out.clone(),
            denom: denom_out.to_string(),
        })
    }

    pub fn right_swap(&self, amount_in: Coin, amount_out: Coin) -> StdResult<Coin> {
        let asset_in = self.pool.clone().find_asset_by_denom(&amount_in.denom)?;
        let asset_out = self.pool.clone().find_asset_by_denom(&amount_out.denom)?;

        let balance_in = Decimal::from_ratio(asset_in.balance.amount, Uint128::one());
        let weight_in = Decimal::from_ratio(asset_in.weight, Uint128::new(100));
        let weight_out = Decimal::from_ratio(asset_out.weight, Uint128::new(100));

        let numerator = Decimal::from_ratio(asset_out.balance.amount, Uint128::one());
        let power = weight_out / weight_in;
        let denominator = Decimal::from_ratio(
            asset_out
                .balance
                .amount
                .checked_sub(amount_out.amount)
                .unwrap_or_default(),
            Uint128::one(),
        );
        let base = numerator / denominator;

        let factor = decimal_to_f64(base).powf(decimal_to_f64(power)) * 1e18;
        let amount_required = (balance_in
            * Decimal::from_ratio(factor as u128, Uint128::from(1e18 as u64)))
        .to_uint_ceil();

        if amount_in.amount < amount_required {
            return Err(StdError::GenericErr {
                msg: "right swap failed: insufficient amount".to_string(),
            });
        }

        Ok(Coin {
            amount: amount_required,
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

    pub fn invariant(&self) -> f64 {
        let mut v = 1.0;
        let mut total_balance = Decimal::zero();
        for asset in self.pool.clone().assets {
            let decimal = 10i64.pow(asset.decimal as u32);
            // let decimal = Decimal::new(Uint128::from(power as u64));
            total_balance +=
                Decimal::from_ratio(asset.balance.amount.clone(), Uint128::from(decimal as u64));
        }
        for asset in self.pool.clone().assets {
            let w = asset.weight as f64 / 100.0;
            let decimal = 10i64.pow(asset.decimal as u32);
            // let decimal = Decimal::from_int().unwrap();
            let balance =
                Decimal::from_ratio(asset.balance.amount.clone(), Uint128::from(decimal as u64));
            v *= decimal_to_f64(balance).powf(w);
        }
        v
    }

    pub fn lp_price(&self) -> f64 {
        let invariant = self.invariant();
        let supply_amount = self.pool.supply.amount.u128();
        let lp_price = invariant / supply_amount as f64;
        lp_price
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
