use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cw_storage_plus::Item;

use std::cmp::Ordering;
use std::collections::VecDeque;
use crate::staking::{withdraw_msg, undelegate_msg};
use cosmwasm_std::{CosmosMsg, StdError, StdResult, Uint128, CustomQuery, QuerierWrapper};

pub const VALIDATOR_SET: Item<ValidatorSet> = Item::new("validator_set");

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct ValidatorResponse {
    pub address: String,
    pub staked: Uint128,
}

#[derive(Eq, PartialEq, Serialize, Deserialize, Debug, Clone, Default)]
pub struct Validator {
    pub address: String,
    pub staked: Uint128,
}

impl PartialOrd for Validator {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Validator {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.staked)
            .cmp(&(other.staked))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ValidatorSet {
    pub validators: VecDeque<Validator>,
}

impl ValidatorSet {
    pub fn to_query_response(&self) -> Vec<ValidatorResponse> {
        self.validators
            .clone()
            .into_iter()
            .map(|v| ValidatorResponse {
                address: v.address,
                staked: Uint128::from(v.staked),
            })
            .collect()
    }

    // pub fn next_to_unbond(&self) -> Option<&Validator> {
    //     if self.validators.is_empty() {
    //         return None;
    //     }
    //     self.validators.front()
    // }

    /// Remove validator from validator-set if staked value is zero
    pub fn remove(&mut self, address: &str, force: bool) -> StdResult<Option<Validator>> {
        let pos = self.exists(address);
        if pos.is_none() {
            return Err(StdError::generic_err(format!(
                "Failed to remove validator: {}, doesn't exist",
                address
            )));
        }

        let val = self.validators.get(pos.unwrap()).ok_or_else(|| {
            StdError::generic_err(format!(
                "Failed to remove validator: {}, failed to get from validator list",
                address
            ))
        })?;

        if !force && val.staked.u128() != 0 {
            return Err(StdError::generic_err(format!(
                "Failed to remove validator: {}, you need to undelegate {}uscrt first or set the flag force=true",
                address, val.staked
            )));
        }

        Ok(self.validators.remove(pos.unwrap()))
    }

    /// Returns total staked value
    pub fn total_staked(&self) -> u128 {
        self.validators.iter().map(|val| val.staked.u128()).sum()
    }

    /// Add validator to validator structure
    pub fn add(&mut self, address: String) {
        if self.exists(&address).is_none() {
            self.validators.push_back(Validator {
                address,
                staked: Uint128::from(0u128),
            })
        }
    }

    // trigger unbond from a given validator address
    pub fn unbond_from(&mut self, address: &str, to_unbond: u128) -> StdResult<()> {
        if self.validators.is_empty() {
            return Err(StdError::generic_err(
                "Failed to get validator to unbond - validator set is empty",
            ));
        }

        for val in self.validators.iter_mut() {
            if val.address == address {
                val.staked = Uint128::from(val.staked.u128().saturating_sub(to_unbond));
                return Ok(());
            }
        }

        Err(StdError::generic_err(
            "Failed to get validator to stake - validator not found",
        ))
    }

    // trigger unbond from validator with most staked
    pub fn unbond_from_largest(&mut self, to_unbond: u128) -> StdResult<String> {
        if self.validators.is_empty() {
            return Err(StdError::generic_err(
                "Failed to get validator to unbond - validator set is empty",
            ));
        }

        let mut val = self.validators.front_mut().unwrap();
        val.staked = Uint128::from(val.staked.u128().saturating_sub(to_unbond));
        Ok(val.address.clone())
    }

    // Stake at validator with least staked asset
    // Returns validator address
    pub fn stake_with_least(&mut self, to_stake: u128) -> StdResult<String> {
        if self.validators.is_empty() {
            return Err(StdError::generic_err(
                "Failed to get validator to stake - validator set is empty",
            ));
        }

        let val = self.validators.back_mut().unwrap();
        val.staked = Uint128::from(val.staked.u128() + to_stake);
        Ok(val.address.clone())
    }

    // Stake at a given validator address
    pub fn stake_at(&mut self, address: &str, to_stake: u128) -> StdResult<()> {
        if self.validators.is_empty() {
            return Err(StdError::generic_err(
                "Failed to get validator to stake - validator set is empty",
            ));
        }

        for val in self.validators.iter_mut() {
            if val.address == address {
                val.staked = Uint128::from(val.staked.u128() + to_stake);
                return Ok(());
            }
        }

        Err(StdError::generic_err(
            "Failed to get validator to stake - validator not found",
        ))
    }

    pub fn exists(&self, address: &str) -> Option<usize> {
        self.validators.iter().position(|v| v.address == address)
    }

    // call this after every stake or unbond call
    // sorts validators by amount of SCRT staked
    pub fn rebalance(&mut self) {
        if self.validators.len() < 2 {
            return;
        }

        self.validators.make_contiguous().sort_by(|a, b| b.cmp(a));
    }

    pub fn query_rewards<Q: CustomQuery>(
        &self,
        querier: QuerierWrapper<Q>,
        address: String
    ) -> StdResult<u128> {
        let mut total_rewards = 0u128;
        for val in self.validators.iter() {
            if let Some(query) = querier.query_delegation(address.clone(), val.address.clone())? {
                for reward in query.accumulated_rewards.iter() {
                    if reward.denom == "uside" {
                        total_rewards += reward.amount.u128();
                    }
                }
            }
        }
        Ok(total_rewards)
    }

    pub fn query_rewards_validator<Q: CustomQuery>(
        &self,
        querier: QuerierWrapper<Q>,
        address: String,
        validator: String
    ) -> StdResult<u128> {
        let mut total_rewards = 0u128;
        if let Some(query) = querier.query_delegation(address.clone(), validator)? {
            for reward in query.accumulated_rewards.iter() {
                if reward.denom == "uside" {
                    total_rewards += reward.amount.u128();
                }
            }
        }
        Ok(total_rewards)
    }

    pub fn withdraw_rewards_messages(&self) -> Vec<CosmosMsg> {
        self.validators
            .iter()
            .filter(|&val| val.staked.u128() > 0)
            .map(|val| withdraw_msg(&val.address))
            .collect()
    }

    pub fn unbond_all(&self) -> Vec<CosmosMsg> {
        self.validators
            .iter()
            .filter(|&val| val.staked.u128() > 0)
            .map(|val| undelegate_msg(&val.address, val.staked.u128()))
            .collect()
    }

    pub fn zero(&mut self) {
        if self.validators.is_empty() {
            return;
        }

        for val in self.validators.iter_mut() {
            val.staked = Uint128::from(0u128);
        }
    }
}
