use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use halo_farm::state::TokenInfo;

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    pub farm_code_id: u64,
}

// We define a custom struct for each query response
#[cw_serde]
pub struct ConfigResponse {
    pub owner: String,
    pub farm_code_id: u64,
}

// We define a custom struct for storing phases info
#[cw_serde]
pub struct FactoryFarmInfo {
    pub staked_token: Addr,
    pub reward_token: TokenInfo,
    pub start_time: u64,
    pub end_time: u64,
    pub phases_limit_per_user: Option<Uint128>,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const FARMS: Map<u64, FactoryFarmInfo> = Map::new("farms");
pub const NUMBER_OF_FARMS: Item<u64> = Item::new("NUMBER_OF_FARMS");
