use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use serde::{Deserialize, Serialize};

// use std::convert::TryFrom;

pub const USER_CLAIMABLE: Item<UserClaimable> = Item::new("user_claimable");

pub const QUEUE_WINDOW_AMOUNT: Map<&Addr, Uint128> = Map::new("queue_window");
pub const BQUEUE_WINDOW_AMOUNT: Map<&Addr, Uint128> = Map::new("bqueue_window");
pub const ONGOING_WITHDRAWS_AMOUNT: Map<(&str, &Addr), Uint128> = Map::new("ongoing_withdraws");
pub const USER_CLAIMABLE_AMOUNT: Map<&Addr, Uint128> = Map::new("user_claimable_map");

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct QueueWindow {
    pub id: u64,
    pub total_sejuno: Uint128,
    pub total_bjuno: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct OngoingWithdrawWindow {
    pub id: u64,
    pub time_to_mature_window: u64,
    pub total_juno: Uint128,
    pub total_sejuno: Uint128,
    pub total_bjuno: Uint128
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct UserClaimable {
    pub total_juno: Uint128,
    // storage: PrefixedStorage<'a, S>,
}

// This struct refactors out the readonly methods that we need for `Balances` and `ReadonlyBalances`
// in a way that is generic over their mutability.
//
// This was the only way to prevent code duplication of these methods because of the way
// that `ReadonlyPrefixedStorage` and `PrefixedStorage` are implemented in `cosmwasm-std`
// struct ReadonlyUserClaimableImpl<'a, S: ReadonlyStorage>(&'a S);

// impl<'a, S: ReadonlyStorage> ReadonlyUserClaimableImpl<'a, S> {
//     pub fn account_amount(&self, account: &String) -> u128 {
//         let account_bytes = account.as_bytes();
//         let result = self.0.get(account_bytes);
//         match result {
//             // This unwrap is ok because we know we stored things correctly
//             Some(balance_bytes) => slice_to_u128(&balance_bytes).unwrap(),
//             None => 0,
//         }
//     }
// }

// Converts 16 bytes value into u128
// Errors if data found that is not 16 bytes
// fn slice_to_u128(data: &[u8]) -> StdResult<u128> {
//     match <[u8; 16]>::try_from(data) {
//         Ok(bytes) => Ok(u128::from_be_bytes(bytes)),
//         Err(_) => Err(StdError::generic_err(
//             "Corrupted data found. 16 byte expected.",
//         )),
//     }
// }

// pub struct ReadonlyUserClaimable<'a, S: ReadonlyStorage> {
//     storage: ReadonlyPrefixedStorage<'a, S>,
// }

// impl<'a, S: ReadonlyStorage> ReadonlyUserClaimable<'a, S> {
//     pub fn from_storage(storage: &'a S) -> Self {
//         Self {
//             storage: ReadonlyPrefixedStorage::new(USER_CLAIMABLE, storage),
//         }
//     }

//     fn as_readonly(&self) -> ReadonlyUserClaimableImpl<ReadonlyPrefixedStorage<S>> {
//         ReadonlyUserClaimableImpl(&self.storage)
//     }

//     pub fn account_amount(&self, account: &String) -> u128 {
//         self.as_readonly().account_amount(account)
//     }
// }

// impl<'a, S: Storage> UserClaimable<'a, S> {
//     pub fn from_storage(storage: &'a mut S) -> Self {
//         Self {
//             storage: PrefixedStorage::new(USER_CLAIMABLE, storage)
//         }
//     }

//     fn as_readonly(&self) -> ReadonlyUserClaimableImpl<PrefixedStorage<S>> {
//         ReadonlyUserClaimableImpl(&self.storage)
//     }

//     pub fn balance(&self, account: &String) -> u128 {
//         self.as_readonly().account_amount(account)
//     }

//     pub fn get_total(&self) -> u128 {
//         self.as_readonly().account_amount(&"total_juno".to_string())
//     }

//     pub fn set_total(&mut self, amount: u128) {
//         self.storage.set("total_juno".as_bytes(), &amount.to_be_bytes())
//     }

//     pub fn set_account_balance(&mut self, account: &String, amount: u128) {
//         self.storage.set(account.as_bytes(), &amount.to_be_bytes())
//     }
// }
