use cosmwasm_std::{entry_point, to_binary, Binary, Deps, Env, StdResult};

use crate::msg::QueryMsg;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Contract {} => to_binary(&query_contract(deps)?),
        QueryMsg::TotalVolume {} => to_binary(&query_total_volume(deps, env)?),
        QueryMsg::TotalVolumeAt { timestamp } => {
            to_binary(&query_total_volume_at(deps, timestamp)?)
        } //QueryMsg::VolumeInterval { start, end } => to_binary(&query_total_volume_interval(deps, start, end)?),
    }
}

fn query_contract(deps: Deps) -> StdResult<String> {
    let config = CONFIG.load(deps.storage)?;

    Ok(config.contract_address)
}

fn query_total_volume(deps: Deps, env: Env) -> StdResult<Observation> {
    let res = binary_search(deps, env.block.time.nanos())?;
    Ok(OBSERVATIONS.load(deps.storage, res)?)
}

fn query_total_volume_at(deps: Deps, timestamp: u64) -> StdResult<Observation> {
    let res = binary_search(deps, timestamp)?;
    Ok(OBSERVATIONS.load(deps.storage, res)?)
}
