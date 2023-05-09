use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Api, CanonicalAddr, Order, StdResult, Storage, Uint128};
use cw_storage_plus::Item;
use std::fmt;

pub const POOL_INFO: Item<PoolInfo> = Item::new("pool_info");

// RewardTokenInfo is an enum that can be either a Token or a NativeToken
#[cw_serde]
pub enum RewardTokenInfo {
    Token { contract_addr: String },
    NativeToken { denom: String },
}

impl fmt::Display for RewardTokenInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RewardTokenInfo::NativeToken { denom } => write!(f, "{}", denom),
            RewardTokenInfo::Token { contract_addr } => write!(f, "{}", contract_addr),
        }
    }
}

#[cw_serde]
pub enum RewardTokenInfoRaw {
    Token { contract_addr: CanonicalAddr },
    NativeToken { denom: String },
}

impl RewardTokenInfo {
    pub fn is_token(&self) -> bool {
        match self {
            RewardTokenInfo::Token { .. } => true,
            _ => false,
        }
    }

    pub fn is_native_token(&self) -> bool {
        match self {
            RewardTokenInfo::NativeToken { .. } => true,
            _ => false,
        }
    }

    pub fn to_raw(&self, api: &dyn Api) -> StdResult<RewardTokenInfoRaw> {
        match self {
            RewardTokenInfo::NativeToken { denom } => Ok(RewardTokenInfoRaw::NativeToken {
                denom: denom.to_string(),
            }),
            RewardTokenInfo::Token { contract_addr } => Ok(RewardTokenInfoRaw::Token {
                contract_addr: api.addr_canonicalize(contract_addr.as_str())?,
            }),
        }
    }
}

// We define a custom struct for each query response
#[cw_serde]
pub struct PoolInfo {
    pub staked_token: String,
    pub reward_token: RewardTokenInfo,
    pub reward_per_second: Uint128,
    pub start_time: u64,
    pub end_time: u64,
}

// We define a custom struct for each query response
#[cw_serde]
pub struct PoolResponse {
    pub staked_token: String,
    pub total_share: Uint128,
}
