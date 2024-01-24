use cosmwasm_std::{QuerierWrapper, StdResult};

use crate::msg::ParamResponse;
use crate::query::{SideQuery, SideQueryWrapper};
use crate::route::SideRoute;

pub struct SideQuerier<'a> {
    querier: &'a QuerierWrapper<'a, SideQueryWrapper>,
}

impl<'a> SideQuerier<'a> {
    pub fn new(querier: &'a QuerierWrapper<SideQueryWrapper>) -> Self {
        SideQuerier { querier }
    }

    // Gmm
    pub fn query_params(&self) -> StdResult<ParamResponse> {
        let request = SideQueryWrapper {
            route: SideRoute::Gmm,
            query_data: SideQuery::Params {}
        };

        let res: ParamResponse = self.querier.query(&request.into())?;
        Ok(res)
    }
}