use crate::ContractError;
use cosmwasm_std::{
    CosmosMsg, Env, Uint128, DepsMut, MessageInfo,
    Response, WasmMsg, to_binary, StdError, Addr
};

use crate::msg::{AirdropMessage1, AirdropMessage2, AirdropMessage3};
use crate::types::config::{CONFIG};

/**
 * Claim airdrop
 */
pub fn claim_airdrop_merkle_1(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    claim_address: Addr,
    stage: u8,
    amount: Uint128,
    proof: Vec<String>
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut messages: Vec<CosmosMsg> = vec![];

    if info.sender != config.admin {
        return Err(ContractError::Std(StdError::generic_err(
            "Only admin can claim airdrop"
        )));
    }

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: claim_address.to_string(),
        msg: to_binary(&AirdropMessage1::Claim {
            stage: stage,
            amount: amount,
            proof: proof
        })?,
        funds: vec![],
    }));

    Ok(Response::new().add_messages(messages).add_attribute("action", "claim_airdrop"))
}

/**
 * Claim airdrop
 */
pub fn claim_airdrop_merkle_2(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    claim_address: Addr,
    amount: Uint128,
    proof: Vec<String>
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut messages: Vec<CosmosMsg> = vec![];

    if info.sender != config.admin {
        return Err(ContractError::Std(StdError::generic_err(
            "Only admin can claim airdrop"
        )));
    }

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: claim_address.to_string(),
        msg: to_binary(&AirdropMessage2::Claim {
            amount: amount,
            proof: proof
        })?,
        funds: vec![],
    }));

    Ok(Response::new().add_messages(messages).add_attribute("action", "claim_airdrop"))
}

pub fn claim_airdrop(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    claim_address: Addr,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut messages: Vec<CosmosMsg> = vec![];

    if info.sender != config.admin {
        return Err(ContractError::Std(StdError::generic_err(
            "Only admin can claim airdrop"
        )));
    }

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: claim_address.to_string(),
        msg: to_binary(&AirdropMessage3::Claim {
        })?,
        funds: vec![],
    }));

    Ok(Response::new().add_messages(messages).add_attribute("action", "claim_airdrop"))
}