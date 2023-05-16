use cosmwasm_schema::cw_serde;
use cw_storage_plus::{Bound, Item, Map};
use cosmwasm_std::{Api, CanonicalAddr, Order, StdResult, Storage, Uint128};
use cw_utils::Expiration;
use halo_pool::state::{PoolInfo, RewardTokenInfo};
use std::fmt;

#[cw_serde]
pub struct Config {
    pub owner: CanonicalAddr,
    pub pool_code_id: u64,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const POOLS: Map<&[u8], PoolsInfo> = Map::new("pools");

#[cw_serde]
pub struct TmpPoolInfo {
    pub pool_key: Vec<u8>,
    // LP Token infos
    pub asset_infos: String,
}

// We define a custom struct for storing pools info
#[cw_serde]
pub struct PoolsInfo {
    pub staked_token: String,
    pub reward_token: RewardTokenInfo,
    pub start_time: u64,
    pub end_time: u64,
}

pub const TMP_POOL_INFO: Item<TmpPoolInfo> = Item::new("tmp_pool_info");

pub fn pool_key(asset_infos: String) -> Vec<u8> {
    let mut asset_infos = asset_infos.split(',').collect::<Vec<&str>>();
    asset_infos.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));

    asset_infos[0].as_bytes().to_vec()
}

// We define a custom struct for each query response
#[cw_serde]
pub struct ConfigResponse {
    pub owner: String,
    pub pool_code_id: u64,
}
