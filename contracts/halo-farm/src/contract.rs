use std::env;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, Uint128,
};

use cw2::set_contract_version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:halo-farm";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::{
        Config, FarmInfo, PhaseInfo, CONFIG, FARM_INFO,
    },
    execute::{
        execute_add_reward_balance, execute_deposit, execute_withdraw, execute_harvest,
        execute_add_phase, execute_activate_phase, execute_remove_phase
    },
    query::{query_farm_info, query_pending_reward, query_total_lp_token_staked, query_staker_info},
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        farm_owner: msg.farm_owner,
    };

    // Init phase info
    let phase_info = PhaseInfo {
        start_time: msg.start_time,
        end_time: msg.end_time,
        whitelist: msg.whitelist,
        reward_balance: Uint128::zero(),
        last_reward_time: msg.start_time,
        accrued_token_per_share: Decimal::zero(),
    };

    // Init first phase info
    FARM_INFO.save(
        deps.storage,
        &FarmInfo {
            staked_token: msg.staked_token.clone(),
            reward_token: msg.reward_token.clone(),
            current_phase_index: 0u64,
            phases_info: vec![phase_info],
            phases_limit_per_user: msg.phases_limit_per_user,
            staked_token_balance: Uint128::zero(),
        },
    )?;

    // Save config
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attributes([
        ("method", "instantiate"),
        ("staked_token", msg.staked_token.as_ref()),
        ("reward_token", &msg.reward_token.to_string()),
        ("start_time", &msg.start_time.to_string()),
        ("end_time", &msg.end_time.to_string()),
        (
            "phases_limit_per_user",
            &msg.phases_limit_per_user
                .unwrap_or(Uint128::zero())
                .to_string(),
        ),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AddRewardBalance {
            phase_index,
            amount,
        } => execute_add_reward_balance(deps, env, info, phase_index, amount),
        ExecuteMsg::Deposit { amount } => execute_deposit(deps, env, info, amount),
        ExecuteMsg::Withdraw { amount } => execute_withdraw(deps, env, info, amount),
        ExecuteMsg::Harvest {} => execute_harvest(deps, env, info),
        ExecuteMsg::AddPhase {
            new_start_time,
            new_end_time,
            whitelist,
        } => execute_add_phase(deps, env, info, new_start_time, new_end_time, whitelist),
        ExecuteMsg::ActivatePhase {} => execute_activate_phase(deps, env, info),
        ExecuteMsg::RemovePhase { phase_index } => execute_remove_phase(deps, info, phase_index),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Farm {} => Ok(to_binary(&query_farm_info(deps)?)?),
        QueryMsg::PendingReward { address } => {
            Ok(to_binary(&query_pending_reward(deps, env, address)?)?)
        }
        QueryMsg::TotalStaked {} => Ok(to_binary(&query_total_lp_token_staked(deps)?)?),
        QueryMsg::StakerInfo { address } => Ok(to_binary(&query_staker_info(deps, address)?)?),
    }
}
