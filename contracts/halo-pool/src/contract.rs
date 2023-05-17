#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, to_binary, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, SubMsg, Uint128, WasmMsg,
};

use cw2::set_contract_version;
use cw20::Cw20ExecuteMsg;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:halo-pool";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

use crate::{
    error::ContractError,
    formulas::calc_reward,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::{
        PoolInfo, RewardTokenAsset, RewardTokenInfo, LAST_REWARD_TIME, POOL_INFO, STAKERS_INFO,
    },
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let pool_info = &PoolInfo {
        staked_token: deps.api.addr_validate(&msg.staked_token)?.to_string(),
        reward_token: msg.reward_token.clone(),
        reward_per_second: Uint128::zero(), // this will be updated when admin adding reward balance
        start_time: msg.start_time,
        end_time: msg.end_time,
        whitelist: msg.whitelist,
    };

    POOL_INFO.save(deps.storage, pool_info)?;

    Ok(Response::new().add_attributes([
        ("action", "instantiate"),
        ("staked_token", &msg.staked_token),
        ("reward_token", &msg.reward_token.to_string()),
        ("start_time", &msg.start_time.to_string()),
        ("end_time", &msg.end_time.to_string()),
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
        ExecuteMsg::AddRewardBalance { asset } => {
            execute_add_reward_balance(deps, env, info, asset)
        }
        ExecuteMsg::Deposit { amount } => execute_deposit(deps, env, info, amount),
        ExecuteMsg::Withdraw { amount } => execute_withdraw(deps, env, info, amount),
        ExecuteMsg::Harvest {} => execute_harvest(deps, env, info),
    }
}

pub fn execute_add_reward_balance(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset: RewardTokenAsset,
) -> Result<Response, ContractError> {
    // Get pool info
    let pool_info: PoolInfo = POOL_INFO.load(deps.storage)?;

    // check the message sender is the whitelisted address
    if !pool_info.whitelist.contains(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    // check the balance of native token is sent with the message
    asset.assert_sent_native_token_balance(&info)?;

    let mut res = Response::new();

    // Add reward balance to the pool
    // When creating a new pool, sender must add balance amount of reward_token
    // equivalent to “reward_per_second*(end_time - start_time)” to the new pool address
    // that created from CreatePool msg.
    // Match reward token type:
    // 1. If reward token is native token, sender must add balance amount of native token
    //    to the new pool address by sending via funds when calling this msg.
    // 2. If reward token is cw20 token, sender must add balance amount of cw20 token
    //    to the new pool address by calling cw20 contract transfer_from method.

    if let RewardTokenInfo::Token { contract_addr } = pool_info.reward_token.clone() {
        let transfer = SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                owner: info.sender.to_string(),
                recipient: env.contract.address.to_string(),
                amount: asset.amount,
            })?,
            funds: vec![],
        }));
        res = res.add_submessage(transfer);
    }

    // Update reward_per_second base on new reward balance
    let current_time = env.block.time;
    let reward_amount = calc_reward(&pool_info, current_time.seconds());
    let new_reward_per_second = reward_amount + asset.amount;
    let new_pool_info = PoolInfo {
        staked_token: pool_info.staked_token,
        reward_token: pool_info.reward_token,
        reward_per_second: new_reward_per_second,
        start_time: pool_info.start_time,
        end_time: pool_info.end_time,
        whitelist: pool_info.whitelist,
    };

    // Update last reward time to start time
    LAST_REWARD_TIME.save(deps.storage, &pool_info.start_time)?;

    // Save pool info
    POOL_INFO.save(deps.storage, &new_pool_info)?;

    res = res.add_attribute("method", "add_reward_balance");

    Ok(res)
}

pub fn execute_deposit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let pool_info: PoolInfo = POOL_INFO.load(deps.storage)?;
    // get staker info
    let mut staker_info = STAKERS_INFO
        .may_load(deps.storage, info.sender.clone())?
        .unwrap();
    // if staker already staked before, get the current staker amount
    let _current_staker_amount = staker_info.amount;

    let current_time = env.block.time;
    let reward_amount = calc_reward(&pool_info, current_time.seconds());
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

    // Update staker amount
    staker_info.amount += amount;

    // Update staker info
    STAKERS_INFO.save(deps.storage, info.sender, &staker_info)?;

    res = res
        .add_submessage(transfer)
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
    // get staker info
    let mut staker_info = STAKERS_INFO
        .may_load(deps.storage, info.sender.clone())?
        .unwrap();

    let current_staker_amount = staker_info.amount;

    // only staker can withdraw
    if current_staker_amount == Uint128::zero() {
        return Err(ContractError::Unauthorized {});
    }

    // check staker amount is greater than withdraw amount
    if current_staker_amount < amount {
        return Err(ContractError::InsufficientFunds {});
    }

    let current_time = env.block.time;
    let reward_amount = calc_reward(&pool_info, current_time.seconds());
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

    // Update staker amount
    staker_info.amount -= amount;

    // Update staker info
    STAKERS_INFO.save(deps.storage, info.sender, &staker_info)?;

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
    let current_time = env.block.time;
    let reward_amount = calc_reward(&pool_info, current_time.seconds());

    // Only staker can harvest reward
    let staker_info = STAKERS_INFO
        .may_load(deps.storage, info.sender.clone())?
        .unwrap();
    if staker_info.amount == Uint128::zero() {
        return Err(ContractError::Unauthorized {});
    }

    // Check if there is any reward to harvest
    if reward_amount == Uint128::zero() {
        return Err(ContractError::InvalidZeroAmount {});
    }

    // Transfer reward token to the sender
    let transfer = match pool_info.reward_token {
        RewardTokenInfo::Token { contract_addr } => {
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: info.sender.to_string(),
                    amount: reward_amount,
                })?,
                funds: vec![],
            }))
        }
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
        whitelist: pool_info.whitelist,
    };
    Ok(res)
}
