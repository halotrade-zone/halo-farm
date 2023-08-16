use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;

use crate::state::{ConfigResponse, FactoryFarmInfo};

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
        /// Create farm binary message to instantiate farm contract
        /// This message contains:
        /// staked_token: Addr - Staked LP Token address
        /// reward_token: TokenInfo - Reward Token address (CW20 or Native)
        /// start_time: u64 - Start time
        /// end_time: u64 - End time
        /// phases_limit_per_user: Option<Uint128> - The phases limit of staked tokens per user (0 for unlimited)
        /// whitelist: Addr - Whitelisted addresses
        create_farm_msg: Binary,
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
