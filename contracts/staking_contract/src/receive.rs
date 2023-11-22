use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, from_binary, StdError};
use cw20::Cw20ReceiveMsg;

use crate::{ContractError, msg::Cw20HookMsg, types::config::CONFIG, withdraw::{try_withdraw}, convert::{try_convert_to_sejuno, try_convert_to_bjuno}};

/**
 * Receive cw20 token (seJuno, bJuno) and 
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

    let bjuno_contract_addr = if let Some(b) = config.bjuno_token {
        b
    } else {
        return Err(ContractError::Std(StdError::generic_err(
            "the bJuno token contract must have been registered",
        )));
    };

    let sejuno_contract_addr = if let Some(se) = config.sejuno_token {
        se
    } else {
        return Err(ContractError::Std(StdError::generic_err(
            "the seJuno token contract must have been registered",
        )));
    };

    match from_binary(&_cw20_msg.msg)? {
        Cw20HookMsg::Unbond {} => {
            if info.sender == bjuno_contract_addr {
                try_withdraw(deps, env, info, _cw20_msg,true)
            } else if info.sender == sejuno_contract_addr {
                try_withdraw(deps, env, info, _cw20_msg,false)
            } else {
                Err(ContractError::Std(StdError::generic_err("unauthorized")))
            }
        }
        Cw20HookMsg::Convert {} => {
            if info.sender == bjuno_contract_addr {
                try_convert_to_sejuno(deps, env, info, _cw20_msg)
            } else if info.sender == sejuno_contract_addr {
                try_convert_to_bjuno(deps, env, info, _cw20_msg)
            } else {
                Err(ContractError::Std(StdError::generic_err("unauthorized")))
            }
        }
    }
}
