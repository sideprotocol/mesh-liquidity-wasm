use schemars::JsonSchema;
use cw_storage_plus::Item;
use serde::{Deserialize, Serialize};

pub const CONSTANTS: Item<Constants> = Item::new("constants");

#[derive(Serialize, Debug, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct Constants {
    pub count: i32,
    pub owner: String
}

