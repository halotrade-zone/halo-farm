use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};

use crate::state::{FarmInfo, PendingRewardResponse, StakerInfoResponse, TokenInfo};

#[cw_serde]
pub struct InstantiateMsg {
    /// Staked Token address
    pub staked_token: Addr,
    /// Reward Token address (CW20 or Native)
    pub reward_token: TokenInfo,
    /// Start time
    pub start_time: u64,
    /// End time
    pub end_time: u64,
    // The phases limit of staked tokens per user (0 for unlimited)
    pub phases_limit_per_user: Option<Uint128>,
    // Farm Owner
    pub farm_owner: Addr,
    /// Whitelisted addresses
    pub whitelist: Addr,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Adding reward balance to a phase by whitelisted address
    AddRewardBalance {
        /// Reward phase index
        phase_index: u64,
        /// Reward amount
        amount: Uint128,
    },
    /// Deposit staked tokens and collect reward tokens (if any)
    Deposit {
        amount: Uint128,
    },
    /// Withdraw staked tokens and collect reward tokens (if any)
    Withdraw {
        amount: Uint128,
    },
    // Harvest reward tokens
    Harvest {},
    // // Update Phases Limit Per User
    // UpdatePhasesLimitPerUser {
    //     new_phases_limit_per_user: Uint128,
    // },
    // Add a new farming phase
    AddPhase {
        /// New start time
        new_start_time: u64,
        /// New end time
        new_end_time: u64,
        /// Whitelisted address
        whitelist: Addr,
    },
    // Remove inactive farming phase
    RemovePhase {
        /// Reward phase index
        phase_index: u64,
    },
    // Activate latest farming phase
    ActivatePhase {},
    // /// Removing reward balance from a phase by whitelisted address
    // /// Only can be called when the phase is inactive
    // RemoveRewardBalance {
    //     /// Reward phase index
    //     phase_index: u64,
    // },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(FarmInfo)]
    Farm {},
    #[returns(PendingRewardResponse)]
    PendingReward { address: String },
    #[returns(Uint128)]
    TotalStaked {},
    #[returns(StakerInfoResponse)]
    StakerInfo { address: String },
}
