use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cw20::{Cw20Coin, Logo, MinterResponse, Cw20QueryMsg, TokenInfoResponse};

use cosmwasm_std::{Addr, StdError, StdResult, Uint128, CustomQuery, QuerierWrapper, WasmQuery, to_binary};

pub fn query_total_supply<Q: CustomQuery>(
    querier: QuerierWrapper<Q>,
    token_contract: &Addr,
) -> StdResult<Uint128> {
    let token_info_query = WasmQuery::Smart {
        contract_addr: token_contract.to_string(),
        msg: to_binary(&Cw20QueryMsg::TokenInfo {})?,
    };

    let token_info = querier
        .query(&token_info_query.into())
        .unwrap_or_else(|_| TokenInfoResponse {
            name: "NA".to_string(),
            symbol: "NA".to_string(),
            decimals: 6,
            total_supply: Uint128::from(0u128),
        });

    Ok(token_info.total_supply)
}

#[derive(Serialize, Debug, Deserialize, JsonSchema, Clone, PartialEq)]
pub struct Contract {
    pub address: Addr,
    pub hash: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct InstantiateMarketingInfo {
    pub project: Option<String>,
    pub description: Option<String>,
    pub marketing: Option<String>,
    pub logo: Option<Logo>,
}

/// TokenContract InitMsg
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct TokenInitMsg {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub initial_balances: Vec<Cw20Coin>,
    pub mint: Option<MinterResponse>,
    pub marketing: Option<InstantiateMarketingInfo>,
}

#[allow(clippy::too_many_arguments)]
impl TokenInitMsg {
    
    pub fn get_cap(&self) -> Option<Uint128> {
        self.mint.as_ref().and_then(|v| v.cap)
    }

    pub fn validate(&self) -> StdResult<()> {
        // Check name, symbol, decimals
        if !is_valid_name(&self.name) {
            return Err(StdError::generic_err(
                "Name is not in the expected format (3-50 UTF-8 bytes)",
            ));
        }
        if !is_valid_symbol(&self.symbol) {
            return Err(StdError::generic_err(
                "Ticker symbol is not in expected format [a-zA-Z\\-]{3,12}",
            ));
        }
        if self.decimals > 18 {
            return Err(StdError::generic_err("Decimals must not exceed 18"));
        }
        Ok(())
    }
}

fn is_valid_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.len() < 3 || bytes.len() > 50 {
        return false;
    }
    true
}

fn is_valid_symbol(symbol: &str) -> bool {
    let bytes = symbol.as_bytes();
    if bytes.len() < 3 || bytes.len() > 12 {
        return false;
    }
    for byte in bytes.iter() {
        if (*byte != 45) && (*byte < 65 || *byte > 90) && (*byte < 97 || *byte > 122) {
            return false;
        }
    }
    true
}
