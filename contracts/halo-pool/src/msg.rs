use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};

use crate::state::{PoolInfo, RewardTokenAsset, RewardTokenInfo};

#[cw_serde]
pub struct InstantiateMsg {
    /// Staked Token address
    pub staked_token: String,
    /// Reward Token address (CW20 or Native)
    pub reward_token: RewardTokenInfo,
    /// Start time
    pub start_time: u64,
    /// End time
    pub end_time: u64,
    /// Whitelisted addresses
    pub whitelist: Vec<Addr>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Adding reward balance to pool by whitelisted address
    AddRewardBalance {
        /// Reward amount
        asset: RewardTokenAsset,
    },
    /// Deposit staked tokens and collect reward tokens (if any)
    Deposit {
        amount: Uint128,
    },
    /// Withdraw staked tokens and collect reward tokens (if any), if the pool is inactive, collect all reward tokens
    Withdraw {
        amount: Uint128,
    },
    // Harvest reward tokens
    Harvest {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(PoolInfo)]
    Pool {},
    // TODO: add query for user's reward balance
    // TODO: add query for user's staked balance
}
