use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_binary, Addr, Api, BankMsg, Coin, CosmosMsg, Decimal, MessageInfo, StdError, StdResult,
    SubMsg, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use cw_storage_plus::{Item, Map};
use std::fmt;

#[cw_serde]
pub struct Config {
    pub halo_factory_owner: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");

pub const POOL_INFO: Item<PoolInfo> = Item::new("pool_info");

/// Stores pool info of multiple phases of the same pool.
pub const POOL_INFOS: Item<PoolInfos> = Item::new("pool_infos");

/// Mappping from staker address to staker balance.
pub const STAKERS_INFO: Map<Addr, StakerRewardAssetInfo> = Map::new("stakers_info");

/// Stores the reward token balance of the pool in multiple phases.
pub const PHASES_REWARD_BALANCE: Item<PhasesRewardBalance> = Item::new("phases_reward_balance");

/// Stores the last reward time of the pool in multiple phases.
pub const PHASES_LAST_REWARD_TIME: Item<PhaseLastRewardTime> = Item::new("phases_last_reward_time");

/// Stores the accrued token per share of the pool in multiple phases.
pub const PHASES_ACCRUED_TOKEN_PER_SHARE: Item<PhaseAccruedTokenPerShare> =
    Item::new("phases_accrued_token_per_share");

#[cw_serde]
pub struct StakerRewardAssetInfo {
    pub amount: Uint128,      // How many staked tokens the user has provided.
    pub reward_debt: Uint128, // Reward debt.
    // Phases of the pool that the user has joined.
    // If the user deposit, withdraw or harvest reward, it will be updated to the latest phase
    // to calculate the reward amount correctly if the pool has multiple phases.
    pub joined_phase: u64,
}

#[cw_serde]
pub struct RewardTokenAsset {
    pub info: TokenInfo,
    pub amount: Uint128,
}

impl fmt::Display for RewardTokenAsset {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.info, self.amount)
    }
}

impl RewardTokenAsset {
    pub fn new(info: TokenInfo, amount: Uint128) -> Self {
        Self { info, amount }
    }

    pub fn is_token(&self) -> bool {
        self.info.is_token()
    }

    pub fn is_native_token(&self) -> bool {
        self.info.is_native_token()
    }

    pub fn into_msg(self, recipient: Addr) -> StdResult<CosmosMsg> {
        let amount = self.amount;

        match &self.info {
            TokenInfo::Token { contract_addr } => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: recipient.to_string(),
                    amount,
                })?,
                funds: vec![],
            })),
            TokenInfo::NativeToken { denom } => Ok(CosmosMsg::Bank(BankMsg::Send {
                to_address: recipient.to_string(),
                amount: vec![Coin {
                    amount: self.amount,
                    denom: denom.to_string(),
                }],
            })),
        }
    }

    pub fn into_submsg(self, recipient: Addr) -> StdResult<SubMsg> {
        Ok(SubMsg::new(self.into_msg(recipient)?))
    }

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
    Token { contract_addr: String },
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

#[cw_serde]
pub enum TokenInfoRaw {
    Token { contract_addr: Addr },
    NativeToken { denom: String },
}

impl TokenInfo {
    pub fn is_token(&self) -> bool {
        matches!(self, TokenInfo::Token { .. })
    }

    pub fn is_native_token(&self) -> bool {
        matches!(self, TokenInfo::NativeToken { .. })
    }

    pub fn to_raw(&self, api: &dyn Api) -> StdResult<TokenInfoRaw> {
        match self {
            TokenInfo::NativeToken { denom } => Ok(TokenInfoRaw::NativeToken {
                denom: denom.to_string(),
            }),
            TokenInfo::Token { contract_addr } => Ok(TokenInfoRaw::Token {
                contract_addr: api.addr_validate(contract_addr)?,
            }),
        }
    }
}

// We define a custom struct for each query response
#[cw_serde]
pub struct PoolInfo {
    pub reward_per_second: Decimal,
    pub start_time: u64,
    pub end_time: u64,
    pub pool_limit_per_user: Option<Uint128>,
    pub whitelist: Vec<Addr>,
}

#[cw_serde]
pub struct PoolInfos {
    pub staked_token: String,
    pub reward_token: TokenInfo,
    pub current_phase_index: u64,
    pub pool_infos: Vec<PoolInfo>,
}

#[cw_serde]
pub struct PhasesRewardBalance {
    pub current_phase_index: u64,
    pub reward_balance: Vec<RewardTokenAsset>,
}

#[cw_serde]
pub struct PhaseLastRewardTime {
    pub current_phase_index: u64,
    pub last_reward_time: Vec<u64>,
}

#[cw_serde]
pub struct PhaseAccruedTokenPerShare {
    pub current_phase_index: u64,
    pub accrued_token_per_share: Vec<Decimal>,
}

// We define a custom struct for each query response
#[cw_serde]
pub struct PoolResponse {
    pub staked_token: String,
    pub total_share: Uint128,
}
