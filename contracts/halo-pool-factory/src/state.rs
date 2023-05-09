use cosmwasm_schema::cw_serde;
use cw_storage_plus::{Bound, Item, Map};
use cosmwasm_std::{Api, CanonicalAddr, Order, StdResult, Storage, Uint128};
use halo_pool::state::PoolInfo;
use std::fmt;

#[cw_serde]
pub struct Config {
    pub owner: CanonicalAddr,
    pub pool_code_id: u64,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const POOLS: Map<&[u8], PoolInfo> = Map::new("pools");

#[cw_serde]
pub struct TmpPoolInfo {
    pub pool_key: Vec<u8>,
    // LP Token infos
    pub asset_infos: String,
    pub asset_decimals: u8,
}

pub const TMP_POOL_INFO: Item<TmpPoolInfo> = Item::new("tmp_pool_info");

// We define a custom struct for each query response
#[cw_serde]
pub struct ConfigResponse {
    pub owner: String,
    pub pool_code_id: u64,
}
