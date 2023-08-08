use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};

use crate::state::{ConfigResponse, FactoryFarmInfo};
use halo_farm::state::TokenInfo;

#[cw_serde]
pub struct InstantiateMsg {
    /// Farm code ID
    pub farm_code_id: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// UpdateConfig update relevant code IDs
    UpdateConfig {
        owner: Option<String>,
        farm_code_id: Option<u64>,
    },
    /// CreateFarm instantiates farm contract
    CreateFarm {
        /// Staked LP Token address
        staked_token: Addr,
        /// Reward Token address (CW20 or Native)
        reward_token: TokenInfo,
        /// Start time
        start_time: u64,
        /// End time
        end_time: u64,
        /// The phases limit of staked tokens per user (0 for unlimited)
        phases_limit_per_user: Option<Uint128>,
        /// Whitelisted addresses
        whitelist: Addr,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(FactoryFarmInfo)]
    Farm { farm_id: u64 },
    #[returns(Vec<FactoryFarmInfo>)]
    Farms {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
}
