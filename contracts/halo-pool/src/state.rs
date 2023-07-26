use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, MessageInfo, StdError, StdResult, Uint128};
use cw_storage_plus::{Item, Map};
use std::fmt;

use crate::msg::StakerInfoResponse;

#[cw_serde]
pub struct Config {
    pub halo_factory_owner: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");

/// Stores pool info of multiple phases of the same pool.
pub const POOL_INFOS: Item<PoolInfos> = Item::new("pool_infos");

/// Mappping from staker address to staker balance.
pub const STAKERS_INFO: Map<Addr, StakerInfoResponse> = Map::new("stakers_info_response");

#[cw_serde]
pub struct RewardTokenAsset {
    pub info: TokenInfo,
    pub amount: Uint128,
}

#[cw_serde]
pub struct RewardTokenAssetResponse {
    pub info: TokenInfo,
    pub amount: Uint128,
    pub time_query: u64,
}

impl fmt::Display for RewardTokenAsset {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.info, self.amount)
    }
}

impl RewardTokenAsset {
    pub fn assert_sent_native_token_balance(&self, message_info: &MessageInfo) -> StdResult<()> {
        if let TokenInfo::NativeToken { denom } = &self.info {
            match message_info.funds.iter().find(|x| x.denom == *denom) {
                Some(coin) => {
                    if self.amount == coin.amount {
                        Ok(())
                    } else {
                        Err(StdError::generic_err("Native token balance mismatch between the argument and the transferred"))
                    }
                }
                None => {
                    if self.amount.is_zero() {
                        Ok(())
                    } else {
                        Err(StdError::generic_err("Native token balance mismatch between the argument and the transferred"))
                    }
                }
            }
        } else {
            Ok(())
        }
    }
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
    pub reward_per_second: Decimal,
    pub start_time: u64,
    pub end_time: u64,
    pub whitelist: Addr, // Whitelisted address to add reward balance
    pub reward_balance: Uint128,
    pub last_reward_time: u64,
    pub accrued_token_per_share: Decimal,
    // Total staked token at end time to calculate the reward amount correctly
    // if the pool has multiple phases.
    pub total_staked_at_end_time: Uint128,
}

#[cw_serde]
pub struct PoolInfos {
    pub staked_token: Addr,
    pub reward_token: TokenInfo,
    pub current_phase_index: u64,
    pub phases_info: Vec<PhaseInfo>,
    pub pool_limit_per_user: Option<Uint128>,
}
