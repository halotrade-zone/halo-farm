use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};

use crate::state::{PoolInfo, RewardTokenAsset, TokenInfo};

#[cw_serde]
pub struct InstantiateMsg {
    /// Staked Token address
    pub staked_token: String,
    /// Reward Token address (CW20 or Native)
    pub reward_token: TokenInfo,
    /// Start time
    pub start_time: u64,
    /// End time
    pub end_time: u64,
    // The pool limit of staked tokens per user (0 for unlimited)
    pub pool_limit_per_user: Option<Uint128>,
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
    // Update Pool Limit Per User
    UpdatePoolLimitPerUser {
        new_pool_limit_per_user: Uint128,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(PoolInfo)]
    Pool {},
    #[returns(RewardTokenAsset)]
    PendingReward { address: String },
    #[returns(Uint128)]
    TotalStaked {},
}
