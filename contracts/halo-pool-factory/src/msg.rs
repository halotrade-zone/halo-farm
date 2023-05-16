
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Uint128, Addr};

use halo_pool::state::{RewardTokenInfo, PoolInfo};
use crate::state::{ConfigResponse};

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
		reward_token: RewardTokenInfo,
        /// Reward per second
		reward_per_second: Uint128,
		/// Start time
		start_time: u64,
		/// End time
		end_time: u64,
        /// Whitelisted addresses
        whitelist: Vec<Addr>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(PoolInfo)]
    Pool {
        /// Pool ID
        pool_id: String,
    },
}