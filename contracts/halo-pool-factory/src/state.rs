use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

use halo_pool::state::{TokenInfo, PoolInfos};

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    pub pool_code_id: u64,
}

// We define a custom struct for each query response
#[cw_serde]
pub struct ConfigResponse {
    pub owner: String,
    pub pool_code_id: u64,
}

// We define a custom struct for storing pools info
#[cw_serde]
pub struct FactoryPoolInfo {
    pub staked_token: String,
    pub reward_token: TokenInfo,
    pub start_time: u64,
    pub end_time: u64,
    pub pool_limit_per_user: Option<Uint128>,
}

impl From<PoolInfos> for FactoryPoolInfo {
    fn from(value: PoolInfos) -> Self {
        Self {
            staked_token: value.staked_token,
            reward_token: value.reward_token,
            start_time: value.pool_infos[value.current_phase_index as usize].start_time,
            end_time: value.pool_infos[value.current_phase_index as usize].end_time,
            pool_limit_per_user: value.pool_infos[value.current_phase_index as usize].pool_limit_per_user,
        }
    }
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const POOLS: Map<u64, FactoryPoolInfo> = Map::new("pools");
pub const NUMBER_OF_POOLS: Item<u64> = Item::new("number_of_pools");
