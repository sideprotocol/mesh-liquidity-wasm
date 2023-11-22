use cosmwasm_std::{Addr, StdError, StdResult, Storage, Uint128, Order};

use cw_storage_plus::Item;
use serde::{Deserialize, Serialize};

use rust_decimal::Decimal;
use std::collections::VecDeque;

use crate::msg::PendingClaimsResponse;
use crate::types::withdraw_window::{QueueWindow, OngoingWithdrawWindow, QUEUE_WINDOW_AMOUNT, ONGOING_WITHDRAWS_AMOUNT};
use crate::utils::calc_withdraw;

use crate::types::config::CONFIG;

pub const WINDOW_MANANGER: Item<WindowManager> = Item::new("window_manager");

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct WindowManager {
    pub time_to_close_window: u64,
    pub queue_window: QueueWindow,
    pub ongoing_windows: VecDeque<OngoingWithdrawWindow>,
}

impl WindowManager {
    pub fn add_user_amount_to_active_window(
        &mut self,
        store: &mut dyn Storage,
        user_addr: Addr,
        lsside_amount: Uint128,
    ) -> StdResult<()> {
        if let Some(mut already_stored_amount) = QUEUE_WINDOW_AMOUNT.may_load(store, &user_addr)? { 
            already_stored_amount += lsside_amount;
            QUEUE_WINDOW_AMOUNT.save(store, &user_addr, &already_stored_amount)?;
        } else {
            QUEUE_WINDOW_AMOUNT.save(
                store,
                &user_addr,
                &lsside_amount,
            )?;
        }

        self.queue_window.total_lsside += lsside_amount;

        Ok(())
    }

    pub fn get_user_lsside_in_active_window(
        &self,
        store: &dyn Storage,
        user_addr: Addr,
    ) -> StdResult<Uint128> {
        let mut lsside_amount = Uint128::from(0u128);
        if let Some(lsside_amount_got) = QUEUE_WINDOW_AMOUNT.may_load(store, &user_addr)? { 
            lsside_amount = lsside_amount_got;
        }

        Ok(lsside_amount)
    }

    pub fn advance_window(
        &mut self,
        store: &mut dyn Storage,
        current_time: u64,
        exchange_rate_lsside: Decimal,
        exchange_rate_bjuno: Decimal,
    ) -> StdResult<()> {
        let config = CONFIG.load(store)?;
        let queue_window = self.queue_window.clone();
        let queue_amounts: StdResult<Vec<_>> = QUEUE_WINDOW_AMOUNT.range(store, None, None, Order::Ascending).collect();

        let lsside_to_juno = Uint128::from(calc_withdraw(queue_window.total_lsside, exchange_rate_lsside)?);

        self.ongoing_windows.push_back(OngoingWithdrawWindow {
            id: queue_window.id,
            time_to_mature_window: current_time + config.unbonding_period,
            total_juno: lsside_to_juno,
            total_lsside: queue_window.total_lsside,
        });
        for (user_addr, queue_amt) in queue_amounts?.iter() {
            ONGOING_WITHDRAWS_AMOUNT.save(
                store,
                (&queue_window.id.to_string(), user_addr),
                &Uint128::from(calc_withdraw(*queue_amt, exchange_rate_lsside).unwrap()),
            )?;

            QUEUE_WINDOW_AMOUNT.remove(
                store,
                user_addr,
            );
        }

        self.time_to_close_window = current_time + config.epoch_period;
        self.queue_window = QueueWindow {
            id: queue_window.id+1,
            total_lsside: Uint128::from(0u128),
        };

        Ok(())
    }

    pub fn pop_matured(
        &mut self,
        _store: &dyn Storage,
    ) -> StdResult<OngoingWithdrawWindow> {
        if let Some(matured_window) = self.ongoing_windows.pop_front() {
            Ok(matured_window)
        } else {
            return Err(StdError::generic_err(
                "Previous windows deque empty"
            ));
        }
    }

    pub fn get_user_pending_withdraws(
        &self,
        store: &dyn Storage,
        address: Addr,
    ) -> StdResult<Vec<PendingClaimsResponse>> {
        let mut pending_withdraws: Vec<PendingClaimsResponse> = vec![];

        for ongoing_window in self.ongoing_windows.iter() {
            let window_id = ongoing_window.id.to_string();

            if let Some(user_withdraw_amount) = ONGOING_WITHDRAWS_AMOUNT.may_load(store, (&window_id.to_string(), &address))? {
                if user_withdraw_amount > Uint128::from(0u128) {
                    pending_withdraws.push(PendingClaimsResponse {
                        window_id: ongoing_window.id,
                        claim_time: ongoing_window.time_to_mature_window.clone(),
                        juno_amount: user_withdraw_amount,
                    })
                }
            }
        }
        Ok(pending_withdraws)
    }
}
