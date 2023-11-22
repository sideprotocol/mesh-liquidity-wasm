use crate::types::config::CONFIG;
use crate::types::withdraw_window::{USER_CLAIMABLE, USER_CLAIMABLE_AMOUNT};
use crate::ContractError;
use crate::{msg::ExecuteMsg, state::STATE};
use cosmwasm_std::{
    BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response, StdError, Uint128,
};

/**
 * Transfer matured SIDE from contract to user's wallet.
 */
pub fn claim(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let mut messages: Vec<CosmosMsg> = vec![];
    let mut state = STATE.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;

    if config.paused == true {
        return Err(ContractError::Std(StdError::generic_err(
            "The contract is temporarily paused",
        )));
    }

    let mut user_claimable = USER_CLAIMABLE.load(deps.storage)?;

    let side_amount: Uint128 = Uint128::from(0u128);
    if let Some(side_amount) = USER_CLAIMABLE_AMOUNT.may_load(deps.storage, &_info.sender)? {
        user_claimable.total_side -= side_amount;

        if side_amount > Uint128::from(0u128) {
            // If users wants side amount
            messages.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: _info.sender.to_string(),
                amount: vec![Coin {
                    denom: "uside".to_string(),
                    amount: Uint128::from(side_amount),
                }],
            }));

            state.not_redeemed -= side_amount.clone();
        }

        STATE.save(deps.storage, &state)?;
        USER_CLAIMABLE.save(deps.storage, &user_claimable)?;
        USER_CLAIMABLE_AMOUNT.remove(deps.storage, &_info.sender);
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "claim")
        .add_attribute("account", _info.sender)
        .add_attribute("amount", side_amount))
}
