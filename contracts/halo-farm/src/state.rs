use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::{Item, Map};
use std::fmt;

#[cw_serde]
pub struct Config {
    pub farm_owner: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");

/// Stores info of a farm.
pub const FARM_INFO: Item<FarmInfo> = Item::new("farm_info");

/// Mappping from staker address to staker balance.
pub const STAKERS_INFO: Map<Addr, StakerInfo> = Map::new("stakers_info_response");

#[cw_serde]
pub struct StakerInfo {
    pub amount: Uint128,           // How many staked tokens the user has provided.
    pub reward_debt: Vec<Uint128>, // Store reward debt in multiple phases.
    pub joined_phase: u64,
}

#[cw_serde]
pub struct StakerInfoResponse {
    pub amount: Uint128, // How many staked tokens the user has provided.
    pub joined_phase: u64,
}

#[cw_serde]
pub struct PendingRewardResponse {
    pub info: TokenInfo,
    pub amount: Uint128,
    pub time_query: u64,
}

// TokenInfo is an enum that can be either a Token or a NativeToken
#[cw_serde]
pub enum TokenInfo {
    Token { contract_addr: Addr },
    NativeToken { denom: String },
}

impl fmt::Display for TokenInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TokenInfo::NativeToken { denom } => write!(f, "{}", denom),
            TokenInfo::Token { contract_addr } => write!(f, "{}", contract_addr),
        }
    }
}

// We define a custom struct for each query response
#[cw_serde]
pub struct PhaseInfo {
    pub start_time: u64,
    pub end_time: u64,
    pub whitelist: Addr, // Whitelisted address to add reward balance
    pub reward_balance: Uint128,
    pub last_reward_time: u64,
    pub accrued_token_per_share: Decimal,
}

#[cw_serde]
pub struct FarmInfo {
    pub staked_token: Addr,
    pub reward_token: TokenInfo,
    pub current_phase_index: u64,
    pub phases_info: Vec<PhaseInfo>,
    pub phases_limit_per_user: Option<Uint128>,
    pub staked_token_balance: Uint128, // Total staked token balance in the farm contract
}
