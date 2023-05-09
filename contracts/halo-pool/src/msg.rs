use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

use crate::state::{RewardTokenInfo, PoolInfo};

#[cw_serde]
pub struct InstantiateMsg {
    /// Staked Token address
    pub staked_token: String,
	/// Reward Token address (CW20 or Native)
	pub reward_token: RewardTokenInfo,
    /// Reward per second
	pub reward_per_second: Uint128,
	/// Start time
	pub start_time: u64,
	/// End time
	pub end_time: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
	/// Deposit staked tokens and collect reward tokens (if any)
    Deposit{
		amount: Uint128,
	},
	/// Withdraw staked tokens and collect reward tokens (if any), if the pool is inactive, collect all reward tokens
	Withdraw{
		amount: Uint128,
	},
	// Harvest reward tokens
	Harvest{},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
	#[returns(PoolInfo)]
    Pool {},
}