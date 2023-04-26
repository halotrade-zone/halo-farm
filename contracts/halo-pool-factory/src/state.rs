use cosmwasm_schema::cw_serde;
use cw_storage_plus::{Bound, Item, Map};
use cosmwasm_std::{Api, CanonicalAddr, Order, StdResult, Storage};
use std::fmt;

#[cw_serde]
pub struct Config {
    pub owner: CanonicalAddr,
    pub pool_code_id: u64,
}

pub const CONFIG: Item<Config> = Item::new("config");

// We define a custom struct for each query response
#[cw_serde]
pub struct ConfigResponse {
    pub owner: String,
    pub pool_code_id: u64,
}
