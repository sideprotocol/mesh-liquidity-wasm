use cosmwasm_std::{from_binary, DepsMut, Env, MessageInfo, Response, StdError};
use cw20::Cw20ReceiveMsg;

use crate::{msg::Cw20HookMsg, types::config::CONFIG, withdraw::try_withdraw, ContractError};

/**
 * Receive cw20 token (lsSIDE) and
 * apply action either convert or withdraw
 * read from msg parameters.
 */
pub fn try_receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    _cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if config.paused == true {
        return Err(ContractError::Std(StdError::generic_err(
            "The contract is temporarily paused",
        )));
    }

    let lsside_contract_addr = if let Some(se) = config.ls_side_token {
        se
    } else {
        return Err(ContractError::Std(StdError::generic_err(
            "the lsSIDE token contract must have been registered",
        )));
    };

    match from_binary(&_cw20_msg.msg)? {
        Cw20HookMsg::Unbond {} => {
            if info.sender == lsside_contract_addr {
                try_withdraw(deps, env, info, _cw20_msg)
            } else {
                Err(ContractError::Std(StdError::generic_err("unauthorized")))
            }
        }
    }
}
