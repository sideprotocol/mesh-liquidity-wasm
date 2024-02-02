use cosmwasm_std::{QuerierWrapper, StdResult};

use crate::msg::ParamResponse;
use crate::query::SideQuery;

pub struct SideQuerier<'a> {
    querier: &'a QuerierWrapper<'a, SideQuery>,
}

impl<'a> SideQuerier<'a> {
    pub fn new(querier: &'a QuerierWrapper<SideQuery>) -> Self {
        SideQuerier { querier }
    }

    // Gmm
    pub fn query_params(&self) -> StdResult<ParamResponse> {
        let request = SideQuery::Params {  };

        let res: ParamResponse = self.querier.query(&request.into())?;
        Ok(res)
    }
}