#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, CanonicalAddr, CosmosMsg, Decimal, Deps, DepsMut, Env,
    MessageInfo, Reply, ReplyOn, Response, StdResult, SubMsg, Uint128, WasmMsg, BankMsg, coin,
};
use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg, MinterResponse};
use cw_utils::parse_reply_instantiate_data;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:halo-pool";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

use crate::{msg::{InstantiateMsg, ExecuteMsg, QueryMsg}, state::{PoolInfo, POOL_INFO, RewardTokenInfo}, error::ContractError, formulas::calc_reward};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let pool_info: &PoolInfo = &PoolInfo {
        staked_token: deps.api.addr_validate(&msg.staked_token)?.to_string(),
        reward_token: msg.reward_token.clone(),
        reward_per_second: msg.reward_per_second,
        start_time: msg.start_time,
        end_time: msg.end_time,
    };

    POOL_INFO.save(deps.storage, pool_info)?;

    // When creating a new pool, sender must deposit amount of reward_token
    // equivalent to “reward_per_second*(end time - start_time)” to the new pool address
    // that created from CreatePool msg.
    // Match reward token type
    let transfer = match msg.reward_token {
        RewardTokenInfo::Token { contract_addr } => SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract_addr.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: env.contract.address.to_string(),
                amount: msg.reward_per_second.multiply_ratio(msg.end_time - msg.start_time, 1u64),
            })?,
            funds: vec![],
        })),
        RewardTokenInfo::NativeToken { denom } => SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: env.contract.address.to_string(),
            amount: vec![coin(
                msg.reward_per_second.multiply_ratio(msg.end_time - msg.start_time, 1u64).into(),
                denom,
            )],
        })),
    };

    let res = Response::new()
        .add_submessage(transfer)
        .add_attribute("method", "instantiate");

    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Deposit {amount} => execute_deposit(deps, env, info, amount),
        ExecuteMsg::Withdraw {amount} => execute_withdraw(deps, env, info, amount),
        ExecuteMsg::Harvest {} => execute_harvest(deps, env, info),
    }
}

pub fn execute_deposit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let pool_info: PoolInfo = POOL_INFO.load(deps.storage)?;
    let current_time = env.block.time.seconds();
    let reward_amount = calc_reward(&pool_info, current_time);
    let mut res = Response::new();
    // Harvest reward tokens if any
    if reward_amount > Uint128::zero() {
        let harvest = SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_binary(&ExecuteMsg::Harvest {})?,
            funds: vec![],
        }));
        res = res.add_submessage(harvest);
    };
    // Deposit staked token to the pool
    let transfer = SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: pool_info.staked_token,
        msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
            owner: info.sender.to_string(),
            recipient: env.contract.address.to_string(),
            amount,
        })?,
        funds: vec![],
    }));

    res = res.add_submessage(transfer)
        .add_attribute("method", "deposit");

    Ok(res)
}

pub fn execute_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let pool_info: PoolInfo = POOL_INFO.load(deps.storage)?;
    let current_time = env.block.time.seconds();
    let reward_amount = calc_reward(&pool_info, current_time);
    let mut res = Response::new();

    // Harvest reward tokens if any
    if reward_amount > Uint128::zero() {
        let harvest = SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_binary(&ExecuteMsg::Harvest {})?,
            funds: vec![],
        }));
        res = res.add_submessage(harvest);
    };

    // Withdraw staked token from the pool by using cw20 transfer message
    let withdraw = SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: pool_info.staked_token,
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: info.sender.to_string(),
            amount,
        })?,
        funds: vec![],
    }));

    res = res
        .add_submessage(withdraw)
        .add_attribute("method", "withdraw");

    Ok(res)
}

// Harvest reward token from the pool to the sender
pub fn execute_harvest(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let pool_info: PoolInfo = POOL_INFO.load(deps.storage)?;
    let current_time = env.block.time.seconds();
    let reward_amount = calc_reward(&pool_info, current_time);

    // Check if there is any reward to harvest
    if reward_amount == Uint128::zero() {
        return Err(ContractError::InvalidZeroAmount {});
    }

    // Transfer reward token to the sender
    let transfer = match pool_info.reward_token {
        RewardTokenInfo::Token { contract_addr } => SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract_addr.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount: reward_amount,
            })?,
            funds: vec![],
        })),
        RewardTokenInfo::NativeToken { denom } => SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![coin(reward_amount.into(), denom)],
        })),
    };

    let res = Response::new()
        .add_submessage(transfer)
        .add_attribute("method", "harvest");

    Ok(res)

}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Pool {} => Ok(to_binary(&query_pool_info(deps)?)?),
    }
}

fn query_pool_info(deps: Deps) -> Result<PoolInfo, ContractError> {
    let pool_info: PoolInfo = POOL_INFO.load(deps.storage)?;
    let res = PoolInfo {
        staked_token: pool_info.staked_token,
        reward_token: pool_info.reward_token,
        start_time: pool_info.start_time,
        end_time: pool_info.end_time,
        reward_per_second: pool_info.reward_per_second,
    };
    Ok(res)
}