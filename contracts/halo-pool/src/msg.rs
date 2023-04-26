use cosmwasm_schema::{cw_serde};
use cosmwasm_std::Uint128;

use crate::state::RewardTokenInfo;

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