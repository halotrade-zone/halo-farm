use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};

use crate::state::{ConfigResponse, FactoryPoolInfo};
use halo_farm::state::TokenInfo;

#[cw_serde]
pub struct InstantiateMsg {
    /// Pool code ID
    pub pool_code_id: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// UpdateConfig update relevant code IDs
    UpdateConfig {
        owner: Option<String>,
        pool_code_id: Option<u64>,
    },
    /// CreatePool instantiates pair contract
    CreatePool {
        /// Staked LP Token address
        staked_token: String,
        /// Reward Token address (CW20 or Native)
        reward_token: TokenInfo,
        /// Start time
        start_time: u64,
        /// End time
        end_time: u64,
        /// The pool limit of staked tokens per user (0 for unlimited)
        pool_limit_per_user: Option<Uint128>,
        /// Whitelisted addresses
        whitelist: Vec<Addr>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(FactoryPoolInfo)]
    Pool { pool_id: u64 },
    #[returns(Vec<FactoryPoolInfo>)]
    Pools {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
}
