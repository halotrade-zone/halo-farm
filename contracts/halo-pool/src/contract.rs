#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, to_binary, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, SubMsg, Uint128, WasmMsg, QuerierWrapper, Addr, QueryRequest, WasmQuery, BalanceResponse, BankQuery, Decimal, Uint256, StdError
};

use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg, BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg};


// version info for migration info
const CONTRACT_NAME: &str = "crates.io:halo-pool";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

use crate::{
    error::ContractError,
    formulas::{calc_reward, get_multiplier},
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::{
        PoolInfo, RewardTokenAsset, RewardTokenInfo, LAST_REWARD_TIME, POOL_INFO, STAKERS_INFO, ACCRUED_TOKEN_PER_SHARE, StakerRewardAssetInfo,
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
        reward_per_second: Decimal::zero(), // this will be updated when admin adding reward balance
        start_time: msg.start_time,
        end_time: msg.end_time,
        whitelist: msg.whitelist,
    };

    // Save pool info
    POOL_INFO.save(deps.storage, pool_info)?;

    // Init last reward time to start time
    LAST_REWARD_TIME.save(deps.storage, &msg.start_time)?;

    // Init accrued token per share to zero
    ACCRUED_TOKEN_PER_SHARE.save(deps.storage, &Decimal::zero())?;

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
    let current_time = env.block.time;
    let last_reward_time = LAST_REWARD_TIME.load(deps.storage)?;
    // Get pool info
    let pool_info: PoolInfo = POOL_INFO.load(deps.storage)?;
    // Get accrued token per share
    let accrued_token_per_share = ACCRUED_TOKEN_PER_SHARE.load(deps.storage)?;

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
    let new_reward_per_second = Decimal::from_ratio(asset.amount, pool_info.end_time - pool_info.start_time);
    let new_pool_info = PoolInfo {
        staked_token: pool_info.staked_token,
        reward_token: pool_info.reward_token,
        reward_per_second: new_reward_per_second,
        start_time: pool_info.start_time,
        end_time: pool_info.end_time,
        whitelist: pool_info.whitelist,
    };

    // Save pool info
    POOL_INFO.save(deps.storage, &new_pool_info)?;

    // Update last reward time to start time
    LAST_REWARD_TIME.save(deps.storage, &pool_info.start_time)?;

    // update pool
    let (new_accrued_token_per_share, new_last_reward_time) = update_pool(pool_info.end_time, pool_info.reward_per_second, asset.amount, accrued_token_per_share, current_time.seconds(), last_reward_time);

    // Save accrued token per share
    ACCRUED_TOKEN_PER_SHARE.save(deps.storage, &new_accrued_token_per_share)?;

    // Save last reward time
    LAST_REWARD_TIME.save(deps.storage, &new_last_reward_time)?;

    res = res.add_attribute("method", "add_reward_balance");

    Ok(res)
}

pub fn execute_deposit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // Get current time
    let current_time = env.block.time;
    // Get pool info
    let pool_info: PoolInfo = POOL_INFO.load(deps.storage)?;
    // Get last reward time
    let last_reward_time = LAST_REWARD_TIME.load(deps.storage)?;
    // get staker info
    let mut staker_info = STAKERS_INFO
        .may_load(deps.storage, info.sender.clone())?
        .unwrap_or(StakerRewardAssetInfo {
            amount: Uint128::zero(),
            reward_debt: Uint128::zero(),
        });

    // Get accrued token per share
    let accrued_token_per_share = ACCRUED_TOKEN_PER_SHARE.load(deps.storage)?;

    let mut res = Response::new();

    // Get reward token balance from pool contract if reward token is cw20 token type or get from bank if reward token is native token type
    let reward_token_supply = match pool_info.reward_token {
        RewardTokenInfo::Token { ref contract_addr } => {
            query_token_balance(&deps.querier, contract_addr.to_string(), env.contract.address.clone())?
        }
        RewardTokenInfo::NativeToken { ref denom } => {
            query_balance(&deps.querier, env.contract.address.clone(), denom.to_string())?
        }
    };

    // update pool
    let (new_accrued_token_per_share, new_last_reward_time) = update_pool(pool_info.end_time, pool_info.reward_per_second, reward_token_supply, accrued_token_per_share, current_time.seconds(), last_reward_time);
    // Save accrued token per share
    ACCRUED_TOKEN_PER_SHARE.save(deps.storage, &new_accrued_token_per_share)?;
    // Save last reward time
    LAST_REWARD_TIME.save(deps.storage, &new_last_reward_time)?;

    let reward_amount = (staker_info.amount * new_accrued_token_per_share)
        .checked_sub(staker_info.reward_debt)
        .unwrap_or(Uint128::zero());

    // If there is any reward token in the pool, transfer reward token to the sender
    if reward_amount > Uint128::zero() {
        let transfer_reward = match pool_info.reward_token {
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
        res = res.add_submessage(transfer_reward);
    }

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
    staker_info.reward_debt += staker_info.amount * new_accrued_token_per_share;

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
    // Get current time
    let current_time = env.block.time;
    // Get pool info
    let pool_info: PoolInfo = POOL_INFO.load(deps.storage)?;
    // Get last reward time
    let last_reward_time = LAST_REWARD_TIME.load(deps.storage)?;
    // Only staker can harvest reward
    let mut staker_info = STAKERS_INFO
        .may_load(deps.storage, info.sender.clone())?
        .unwrap();
    // Get accrued token per share
    let accrued_token_per_share = ACCRUED_TOKEN_PER_SHARE.load(deps.storage)?;

    let current_staker_amount = staker_info.amount;

    // Check staker amount is greater than withdraw amount
    if current_staker_amount < amount {
        return Err(ContractError::Std(StdError::generic_err(
            "InsufficientFunds: Withdraw amount exceeds staked amount",
        )));
    }

    // Get reward token balance from pool contract if reward token is cw20 token type or get from bank if reward token is native token type
    let reward_token_supply = match pool_info.reward_token {
        RewardTokenInfo::Token { ref contract_addr } => {
            query_token_balance(&deps.querier, contract_addr.to_string(), env.contract.address)?
        }
        RewardTokenInfo::NativeToken { ref denom } => {
            query_balance(&deps.querier, env.contract.address, denom.to_string())?
        }
    };

    let mut res = Response::new();

    // update pool
    let (new_accrued_token_per_share, new_last_reward_time) = update_pool(pool_info.end_time, pool_info.reward_per_second, reward_token_supply, accrued_token_per_share, current_time.seconds(), last_reward_time);
    // Save accrued token per share
    ACCRUED_TOKEN_PER_SHARE.save(deps.storage, &new_accrued_token_per_share)?;
    // Save last reward time
    LAST_REWARD_TIME.save(deps.storage, &new_last_reward_time)?;

    let reward_amount = (staker_info.amount * new_accrued_token_per_share)
        .checked_sub(staker_info.reward_debt)
        .unwrap_or(Uint128::zero());


    // Transfer reward token to the sender
    let transfer_reward = match pool_info.reward_token {
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
    staker_info.reward_debt += staker_info.amount * new_accrued_token_per_share;

    // Update staker info
    STAKERS_INFO.save(deps.storage, info.sender, &staker_info)?;

    res = res
        .add_submessage(transfer_reward)
        .add_submessage(withdraw)
        .add_attribute("method", "harvest and withdraw");

    Ok(res)
}

// Harvest reward token from the pool to the sender
pub fn execute_harvest(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // Get current time
    let current_time = env.block.time;
    // Get pool info
    let pool_info: PoolInfo = POOL_INFO.load(deps.storage)?;
    // Get last reward time
    let last_reward_time = LAST_REWARD_TIME.load(deps.storage)?;
    // Get accrued token per share
    let accrued_token_per_share = ACCRUED_TOKEN_PER_SHARE.load(deps.storage)?;
    // Only staker can harvest reward
    let mut staker_info = STAKERS_INFO
        .may_load(deps.storage, info.sender.clone())?
        .unwrap();

    // Get reward token balance from pool contract if reward token is cw20 token type or get from bank if reward token is native token type
    let reward_token_supply = match pool_info.reward_token {
        RewardTokenInfo::Token { ref contract_addr } => {
            query_token_balance(&deps.querier, contract_addr.to_string(), env.contract.address)?
        }
        RewardTokenInfo::NativeToken { ref denom } => {
            query_balance(&deps.querier, env.contract.address, denom.to_string())?
        }
    };

    // update pool
    let (new_accrued_token_per_share, new_last_reward_time) = update_pool(pool_info.end_time, pool_info.reward_per_second, reward_token_supply, accrued_token_per_share, current_time.seconds(), last_reward_time);
    // Save accrued token per share
    ACCRUED_TOKEN_PER_SHARE.save(deps.storage, &new_accrued_token_per_share)?;
    // Save last reward time
    LAST_REWARD_TIME.save(deps.storage, &new_last_reward_time)?;

    if staker_info.amount == Uint128::zero() {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: Only staker can harvest reward",
        )));
    }

    let reward_amount = (staker_info.amount * new_accrued_token_per_share)
        .checked_sub(staker_info.reward_debt)
        .unwrap_or(Uint128::zero());

    // Check if there is any reward to harvest
    if reward_amount == Uint128::zero() {
        return Err(ContractError::Std(StdError::generic_err(
            "InsufficientFunds: Reward amount is zero",
        )));
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
    // Update staker reward debt
    staker_info.reward_debt += staker_info.amount * new_accrued_token_per_share;
    // Update staker info
    STAKERS_INFO.save(deps.storage, info.sender, &staker_info)?;

    let res = Response::new()
        .add_submessage(transfer)
        .add_attribute("method", "harvest");

    Ok(res)
}

fn update_pool(
    end_time: u64,
    reward_per_second: Decimal,
    reward_token_supply: Uint128,
    accrued_token_per_share: Decimal,
    current_time: u64,
    last_reward_time: u64,
) -> (Decimal, u64) {

    // If current time is before start time or after end time or before last reward time, return without update pool
    if current_time < last_reward_time {
        return (accrued_token_per_share, last_reward_time);
    }

    // Check if there is any reward token in the pool
    if reward_token_supply == Uint128::zero() {
        // No reward token in the pool, save last reward time and return
        (Decimal::zero(), last_reward_time)
    } else {
        let multiplier = get_multiplier(
            last_reward_time,
            current_time,
            end_time,
        );

        let reward = Decimal::new(multiplier.into()) * reward_per_second;
        let new_accrued_token_per_share = accrued_token_per_share + (reward * Decimal::from_ratio(reward_token_supply, 1_000_000u128));
        (new_accrued_token_per_share, current_time)
    }
}

pub fn query_token_balance(
    querier: &QuerierWrapper,
    contract_addr: String,
    account_addr: Addr,
) -> StdResult<Uint128> {
    let res: Cw20BalanceResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: contract_addr.to_string(),
        msg: to_binary(&Cw20QueryMsg::Balance {
            address: account_addr.to_string(),
        })?,
    }))?;

    // load balance form the token contract
    Ok(res.balance)
}

pub fn query_balance(
    querier: &QuerierWrapper,
    account_addr: Addr,
    denom: String,
) -> StdResult<Uint128> {
    // load price form the oracle
    let balance: BalanceResponse = querier.query(&QueryRequest::Bank(BankQuery::Balance {
        address: account_addr.to_string(),
        denom,
    }))?;
    Ok(balance.amount.amount)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Pool {} => Ok(to_binary(&query_pool_info(deps)?)?),
        QueryMsg::PendingReward { address } => Ok(to_binary(&query_pending_reward(deps, env, address)?)?),
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

fn query_pending_reward(deps: Deps, env: Env, address: Addr) -> Result<RewardTokenAsset, ContractError> {
    // Get current time
    let current_time = env.block.time;
    // Get last reward time
    let last_reward_time = LAST_REWARD_TIME.load(deps.storage)?;
    // Get accrued token per share
    let mut accrued_token_per_share = ACCRUED_TOKEN_PER_SHARE.load(deps.storage)?;
    // Get pool info
    let pool_info: PoolInfo = POOL_INFO.load(deps.storage)?;
    // Get staker info
    let staker_info = STAKERS_INFO
        .may_load(deps.storage, address.clone())?
        .unwrap_or(StakerRewardAssetInfo {
            amount: Uint128::zero(),
            reward_debt: Uint128::zero(),
        });

    // Get reward token balance from pool contract if reward token is cw20 token type or get from bank if reward token is native token type
    let reward_token_supply = match pool_info.reward_token {
        RewardTokenInfo::Token { ref contract_addr } => {
            query_token_balance(&deps.querier, contract_addr.to_string(), env.contract.address)?
        }
        RewardTokenInfo::NativeToken { ref denom } => {
            query_balance(&deps.querier, env.contract.address, denom.to_string())?
        }
    };

    // Check if there is any reward token in the pool
    if reward_token_supply == Uint128::zero() {
        // No reward token in the pool, save last reward time and return
        let res = RewardTokenAsset {
            info: pool_info.reward_token,
            amount: Uint128::zero(),
        };
        return Ok(res);
    } else {
        let multiplier = get_multiplier(
            last_reward_time,
            current_time.seconds(),
            pool_info.end_time,
        );

        let reward = Decimal::new(multiplier.into()) * pool_info.reward_per_second;
        accrued_token_per_share = accrued_token_per_share + (reward * Decimal::from_ratio(reward_token_supply, 1_000_000u128));
    }

    let reward_amount = (staker_info.amount * accrued_token_per_share)
        .checked_sub(staker_info.reward_debt)
        .unwrap_or(Uint128::zero());

    let res = RewardTokenAsset {
        info: pool_info.reward_token,
        amount: reward_amount,
    };

    Ok(res)
}

