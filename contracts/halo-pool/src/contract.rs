use std::env;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, to_binary, Addr, BalanceResponse, BankMsg, BankQuery, Binary, CosmosMsg, Decimal, Deps,
    DepsMut, Env, MessageInfo, QuerierWrapper, QueryRequest, Response, StdError, StdResult, SubMsg,
    Uint128, WasmMsg, WasmQuery,
};

use cw2::set_contract_version;
use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:halo-pool";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

use crate::{
    error::ContractError,
    formulas::{calc_reward_amount, get_multiplier, update_pool},
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::{
        Config, PoolInfo, PoolInfos, RewardTokenAsset, RewardTokenAssetResponse,
        StakerInfoResponse, TokenInfo, CONFIG, POOL_INFOS, STAKERS_INFO,
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

    let config = Config {
        halo_factory_owner: msg.pool_owner,
    };

    // Init pool info
    let pool_info = PoolInfo {
        reward_per_second: Decimal::zero(), // will be updated when admin adding reward balance
        start_time: msg.start_time,
        end_time: msg.end_time,
        pool_limit_per_user: msg.pool_limit_per_user,
        total_staked_at_end_time: Uint128::zero(),
    };

    // Init pool infos
    POOL_INFOS.save(
        deps.storage,
        &PoolInfos {
            staked_token: deps.api.addr_validate(&msg.staked_token)?.to_string(),
            reward_token: msg.reward_token.clone(),
            current_phase_index: 0u64,
            pool_infos: vec![pool_info],
            whitelist: msg.whitelist,
            reward_balance: vec![Uint128::zero()],
            last_reward_time: vec![msg.start_time],
            accrued_token_per_share: vec![Decimal::zero()],
        },
    )?;

    // Save config
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attributes([
        ("action", "instantiate"),
        ("staked_token", &msg.staked_token),
        ("reward_token", &msg.reward_token.to_string()),
        ("start_time", &msg.start_time.to_string()),
        ("end_time", &msg.end_time.to_string()),
        (
            "pool_limit_per_user",
            &msg.pool_limit_per_user
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
        ExecuteMsg::AddRewardBalance { phase_index, asset } => {
            execute_add_reward_balance(deps, env, info, phase_index, asset)
        }
        ExecuteMsg::Deposit { amount } => execute_deposit(deps, env, info, amount),
        ExecuteMsg::Withdraw { amount } => execute_withdraw(deps, env, info, amount),
        ExecuteMsg::Harvest {} => execute_harvest(deps, env, info),
        ExecuteMsg::UpdatePoolLimitPerUser {
            new_pool_limit_per_user,
        } => execute_update_pool_limit_per_user(deps, env, info, new_pool_limit_per_user),
        ExecuteMsg::AddPhase {
            new_start_time,
            new_end_time,
        } => execute_add_phase(deps, env, info, new_start_time, new_end_time),
        ExecuteMsg::ActivatePhase {} => execute_activate_phase(deps, env, info),
        ExecuteMsg::RemovePhase { phase_index } => {
            execute_remove_phase(deps, info, phase_index)
        } // ExecuteMsg::RemoveRewardBalance { phase_index } => {
          //     execute_remove_reward_balance(deps, env, info, phase_index)
          // }
    }
}

pub fn execute_add_reward_balance(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    phase_index: u64,
    asset: RewardTokenAsset,
) -> Result<Response, ContractError> {
    // Check the balance of native token is sent with the message
    asset.assert_sent_native_token_balance(&info)?;
    // Get current time
    let current_time = env.block.time.seconds();
    // Get pool infos
    let mut pool_infos = POOL_INFOS.load(deps.storage)?;

    // If reward token is cw20 token, check the contract address of asset is valid
    if let TokenInfo::Token { contract_addr } = asset.info.clone() {
        if contract_addr != pool_infos.reward_token.to_string() {
            return Err(ContractError::Std(StdError::generic_err(
                "Unauthorized: Invalid reward token contract address",
            )));
        }
    }

    // Get current phase index
    let current_phase_index = pool_infos.current_phase_index;

    // Not allow to add reward balance to finished phase
    if phase_index < current_phase_index {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: Can not add reward balance to finished phase",
        )));
    }

    // Check the message sender is the whitelisted address
    if !pool_infos.whitelist.contains(&info.sender) {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: Sender is not in the whitelisted address",
        )));
    }

    // Get pool info in pool infos
    let pool_info = pool_infos.pool_infos[phase_index as usize].clone();

    // Not allow adding reward balance when current time is greater than start time of the phase
    if current_time > pool_info.start_time {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: Current time is greater than start time of the phase",
        )));
    }

    // Get the reward token balance of the pool in multiple phases.
    let mut phases_reward_balance = pool_infos.reward_balance;
    // Get reward token balance of the pool in current phase
    let mut reward_balance = phases_reward_balance[phase_index as usize];

    // Only allow adding reward balance once
    if reward_balance > Uint128::zero() {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: Can not add reward balance twice",
        )));
    }

    // Get accrued token per share
    let accrued_token_per_share = pool_infos.accrued_token_per_share[phase_index as usize];

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

    if let TokenInfo::Token { contract_addr } = pool_infos.reward_token.clone() {
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

    // Update reward balance
    reward_balance += asset.amount;

    // Save reward balance to phases reward balance
    phases_reward_balance[phase_index as usize] = reward_balance;

    // Update reward_per_second base on new reward balance
    let new_reward_per_second =
        Decimal::from_ratio(reward_balance, pool_info.end_time - pool_info.start_time).floor();

    let new_pool_info = PoolInfo {
        reward_per_second: new_reward_per_second,
        ..pool_info
    };

    // Get staked token balance from pool contract
    let staked_token_supply = query_token_balance(
        &deps.querier,
        pool_infos.staked_token.clone(),
        env.contract.address,
    )?;

    // Update phases reward balance
    pool_infos.reward_balance = phases_reward_balance;

    // Save pool info to pool infos in current pool index
    pool_infos.pool_infos[phase_index as usize] = new_pool_info;
    POOL_INFOS.save(deps.storage, &pool_infos)?;

    // Update last reward time to start time
    pool_infos.last_reward_time[phase_index as usize] = pool_info.start_time;

    // update pool
    let (new_accrued_token_per_share, new_last_reward_time) = update_pool(
        pool_info.end_time,
        pool_info.reward_per_second,
        staked_token_supply,
        accrued_token_per_share,
        current_time,
        pool_infos.last_reward_time[phase_index as usize],
    );

    pool_infos.last_reward_time[phase_index as usize] = new_last_reward_time;
    pool_infos.accrued_token_per_share[phase_index as usize] = new_accrued_token_per_share;

    // Save pool infos
    POOL_INFOS.save(deps.storage, &pool_infos)?;

    res = res.add_attribute("method", "add_reward_balance");

    Ok(res)
}

// pub fn execute_remove_reward_balance(
//     deps: DepsMut,
//     env: Env,
//     info: MessageInfo,
//     phase_index: u64,
// ) -> Result<Response, ContractError> {
//     let current_time = env.block.time;
//     // Get pool info in pool infos
//     let pool_info = POOL_INFOS.load(deps.storage)?.pool_infos[phase_index as usize].clone();
//     let reward_token_balance: Uint128;
//     // Check the message sender is the whitelisted address
//     if !pool_info.whitelist.contains(&info.sender) {
//         return Err(ContractError::Unauthorized {});
//     }

//     // Only can remove reward balance when the pool is inactive
//     if current_time.seconds() < pool_info.start_time || current_time.seconds() > pool_info.end_time {
//         return Err(ContractError::Std(StdError::generic_err(
//             "Unauthorized: Can not remove reward balance when the pool is active",
//         )));
//     }

//     // Transfer all remaining reward token balance to the sender
//     let transfer_reward = match pool_info.reward_token {
//         TokenInfo::Token { contract_addr } => {
//             // Query reward token balance of the pool
//             reward_token_balance = query_token_balance(&deps.querier, contract_addr.clone(), env.contract.address.clone())?;
//             // Check if the reward token balance of the pool is greater than 0
//             if reward_token_balance > Uint128::zero() {
//                 SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
//                     contract_addr,
//                     msg: to_binary(&Cw20ExecuteMsg::Transfer {
//                         recipient: info.sender.to_string(),
//                         amount: reward_token_balance,
//                     })?,
//                     funds: vec![],
//                 }))
//             } else {
//                 return Err(ContractError::Std(StdError::generic_err(
//                     "InvalidZeroAmount: Reward token balance is 0",
//                 )));
//             }
//         }
//         TokenInfo::NativeToken { denom } =>{
//             // Query reward token balance of the pool
//             reward_token_balance = query_balance(&deps.querier, env.contract.address.clone(), denom.clone())?;
//             // Check if the reward token balance of the pool is greater than 0
//             if reward_token_balance > Uint128::zero() {
//                 SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
//                     to_address: info.sender.to_string(),
//                     amount: vec![coin(reward_token_balance.into(), denom)],
//                 }))
//             } else {
//                 return Err(ContractError::Std(StdError::generic_err(
//                     "InvalidZeroAmount: Reward token balance is 0",
//                 )));
//             }
//         }
//     };

//     Ok(Response::new().add_submessage(transfer_reward)
//         .add_attribute("method", "remove_reward_balance")
//         .add_attribute("phase_index", phase_index.to_string())
//         .add_attribute("reward_token_balance", reward_token_balance.to_string())
//     )
// }

pub fn execute_remove_phase(
    deps: DepsMut,
    info: MessageInfo,
    phase_index: u64,
) -> Result<Response, ContractError> {
    // Get pool infos
    let mut pool_infos = POOL_INFOS.load(deps.storage)?;
    // Get current pool index
    let current_phase_index = pool_infos.current_phase_index;

    // Not allow removing activated phase
    if current_phase_index >= phase_index {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: Can not remove activated phase",
        )));
    }

    // Check the message sender is the whitelisted address
    if !pool_infos.whitelist.contains(&info.sender) {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: Sender is not in the whitelisted address",
        )));
    }

    // If the pool already added reward balance, transfer all phase reward token balance to the sender
    if pool_infos.reward_balance[phase_index as usize] > Uint128::zero() {
        // Transfer all remaining reward token balance to the sender
        let transfer_reward = match pool_infos.reward_token.clone() {
            TokenInfo::Token { contract_addr } => SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: info.sender.to_string(),
                    amount: pool_infos.reward_balance[phase_index as usize],
                })?,
                funds: vec![],
            })),
            TokenInfo::NativeToken { denom } => SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: vec![coin(
                    pool_infos.reward_balance[phase_index as usize].into(),
                    denom,
                )],
            })),
        };
        return Ok(Response::new().add_submessage(transfer_reward));
    }

    // Remove phase
    pool_infos.pool_infos.remove(phase_index as usize);
    pool_infos.reward_balance.remove(phase_index as usize);
    pool_infos.last_reward_time.remove(phase_index as usize);
    pool_infos
        .accrued_token_per_share
        .remove(phase_index as usize);

    // Save pool infos
    POOL_INFOS.save(deps.storage, &pool_infos)?;

    Ok(Response::new().add_attribute("method", "remove_phase"))
}

pub fn execute_deposit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // Get current time
    let current_time = env.block.time.seconds();
    // Get pool infos
    let mut pool_infos = POOL_INFOS.load(deps.storage)?;
    // Get current pool index
    let current_phase_index = pool_infos.current_phase_index;
    // Get current pool info in pool infos
    let pool_info = pool_infos.pool_infos[current_phase_index as usize].clone();
    // get staker info
    let mut staker_info = STAKERS_INFO
        .may_load(deps.storage, info.sender.clone())?
        .unwrap_or(StakerInfoResponse {
            amount: Uint128::zero(),
            reward_debt: vec![Uint128::zero(); current_phase_index as usize + 1],
            joined_phase: current_phase_index,
        });

    // Not allow depositing when current time is greater than end time of the phase
    // and less than start time of the phase
    if current_time > pool_info.end_time || current_time < pool_info.start_time
    {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: Current time is not in the range of the phase",
        )));
    }

    // Check pool limit per user
    if let Some(pool_limit_per_user) = pool_info.pool_limit_per_user {
        if staker_info.amount + amount > pool_limit_per_user {
            return Err(ContractError::Std(StdError::generic_err(
                "Unauthorized: Deposit amount exceeds pool limit per user",
            )));
        }
    }

    // Init reward amount
    let mut reward_amount = Uint128::zero();
    // Init new accrued token per share
    let mut new_accrued_token_per_share;
    // Init new last reward time
    let mut new_last_reward_time;
    // Init response
    let mut res = Response::new();

    // If staker has joined previous phases, loops all pool infos to get reward per second from current pool index to staker joined phases
    if staker_info.joined_phase < current_phase_index {
        for i in staker_info.joined_phase..current_phase_index {
            // Get last reward time
            let last_reward_time = pool_infos.last_reward_time[i as usize];
            // Get accrued token per share
            let accrued_token_per_share = pool_infos.accrued_token_per_share[i as usize];
            // Get pool info from pool infos
            let pool_info = POOL_INFOS.load(deps.storage)?.pool_infos[i as usize].clone();
            // Get staked token balance from pool contract previous phases
            let staked_token_supply = pool_info.total_staked_at_end_time;

            // update pool
            (new_accrued_token_per_share, new_last_reward_time) = update_pool(
                pool_info.end_time,
                pool_info.reward_per_second,
                staked_token_supply,
                accrued_token_per_share,
                pool_info.end_time,
                last_reward_time,
            );

            reward_amount += calc_reward_amount(
                staker_info.amount,
                new_accrued_token_per_share,
                staker_info.reward_debt[i as usize],
            );

            // Update staker info
            staker_info.reward_debt[i as usize] = staker_info.amount * new_accrued_token_per_share;
            // Update last reward time
            pool_infos.last_reward_time[i as usize] = new_last_reward_time;
            // Update accrued token per share
            pool_infos.accrued_token_per_share[i as usize] = new_accrued_token_per_share;

            // Save pool infos
            POOL_INFOS.save(deps.storage, &pool_infos)?;
        }
        // // Reset reward debt to 0
        // staker_info.reward_debt = Uint128::zero();
        // Increase length of user reward debt
        staker_info.reward_debt.push(Uint128::zero());
    }

    // Get staked token balance from pool contract
    let staked_token_supply = query_token_balance(
        &deps.querier,
        pool_infos.staked_token.clone(),
        env.contract.address.clone(),
    )?;

    // Get last reward time
    let mut last_reward_time = pool_infos.last_reward_time[current_phase_index as usize];
    // For the first deposit, set last reward time to current time
    if staked_token_supply == Uint128::zero() {
        last_reward_time = current_time;
    }
    // Get accrued token per share
    let accrued_token_per_share = pool_infos.accrued_token_per_share[current_phase_index as usize];

    // update pool
    (new_accrued_token_per_share, new_last_reward_time) = update_pool(
        pool_info.end_time,
        pool_info.reward_per_second,
        staked_token_supply,
        accrued_token_per_share,
        current_time,
        last_reward_time,
    );

    pool_infos.last_reward_time[current_phase_index as usize] = new_last_reward_time;
    pool_infos.accrued_token_per_share[current_phase_index as usize] = new_accrued_token_per_share;

    // Save pool infos
    POOL_INFOS.save(deps.storage, &pool_infos)?;

    reward_amount += calc_reward_amount(
        staker_info.amount,
        new_accrued_token_per_share,
        staker_info.reward_debt[current_phase_index as usize],
    );

    // If there is any reward token in the pool, transfer reward token to the sender
    if reward_amount > Uint128::zero() {
        let transfer_reward = match pool_infos.reward_token {
            TokenInfo::Token { contract_addr } => SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: info.sender.to_string(),
                    amount: reward_amount,
                })?,
                funds: vec![],
            })),
            TokenInfo::NativeToken { denom } => SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: vec![coin(reward_amount.into(), denom)],
            })),
        };
        res = res.add_submessage(transfer_reward);
    }

    // Deposit staked token to the pool
    let transfer = SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: pool_infos.staked_token,
        msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
            owner: info.sender.to_string(),
            recipient: env.contract.address.to_string(),
            amount,
        })?,
        funds: vec![],
    }));

    // Update staker info
    staker_info.amount += amount;
    staker_info.reward_debt[current_phase_index as usize] = staker_info.amount * new_accrued_token_per_share;
    staker_info.joined_phase = current_phase_index;

    STAKERS_INFO.save(deps.storage, info.sender, &staker_info)?;

    res = res
        .add_submessage(transfer)
        .add_attribute("previous_receive_reward_time", last_reward_time.to_string())
        .add_attribute("current_time", current_time.to_string())
        .add_attribute("method", "deposit")
        .add_attribute("deposit_amount", amount.to_string())
        .add_attribute("reward_amount", reward_amount.to_string());

    Ok(res)
}

pub fn execute_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // Get current time
    let current_time = env.block.time.seconds();
    // Get pool infos
    let mut pool_infos = POOL_INFOS.load(deps.storage)?;
    // Get current pool index
    let current_phase_index = pool_infos.current_phase_index;
    // Get Staker info
    let mut staker_info = STAKERS_INFO
        .may_load(deps.storage, info.sender.clone())?
        .unwrap_or(StakerInfoResponse {
            amount: Uint128::zero(),
            reward_debt: vec![Uint128::zero(); current_phase_index as usize + 1],
            joined_phase: current_phase_index,
        });

    if staker_info.amount == Uint128::zero() {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: Only staker can withdraw",
        )));
    }

    // Check staker amount is greater than withdraw amount
    if staker_info.amount < amount {
        return Err(ContractError::Std(StdError::generic_err(
            "InsufficientFunds: Withdraw amount exceeds staked amount",
        )));
    }

    // Init reward amount
    let mut reward_amount = Uint128::zero();
    // Get current pool info in pool infos
    let pool_info = POOL_INFOS.load(deps.storage)?.pool_infos[current_phase_index as usize].clone();
    // Init new accrued token per share
    let mut new_accrued_token_per_share;
    // Init new last reward time
    let mut new_last_reward_time;

    let mut res = Response::new();

    // get staker joined phases
    let staker_joined_phase = staker_info.joined_phase;

    // If staker has joined previous phases, loops all pool infos to get reward per second from current pool index to staker joined phases
    if staker_joined_phase < current_phase_index {
        for i in staker_joined_phase..current_phase_index {
            // Get last reward time
            let last_reward_time = pool_infos.last_reward_time[i as usize];
            // Get accrued token per share
            let accrued_token_per_share = pool_infos.accrued_token_per_share[i as usize];
            // Get pool info from pool infos
            let pool_info = POOL_INFOS.load(deps.storage)?.pool_infos[i as usize].clone();
            // Get staked token balance from pool contract previous phases
            let staked_token_supply = pool_info.total_staked_at_end_time;

            // update pool
            (new_accrued_token_per_share, new_last_reward_time) = update_pool(
                pool_info.end_time,
                pool_info.reward_per_second,
                staked_token_supply,
                accrued_token_per_share,
                pool_info.end_time,
                last_reward_time,
            );

            pool_infos.last_reward_time[i as usize] = new_last_reward_time;
            pool_infos.accrued_token_per_share[i as usize] = new_accrued_token_per_share;

            // Save pool infos
            POOL_INFOS.save(deps.storage, &pool_infos)?;

            reward_amount += calc_reward_amount(
                staker_info.amount,
                new_accrued_token_per_share,
                staker_info.reward_debt[i as usize],
            );
            // Update staker info
            staker_info.reward_debt[i as usize] = staker_info.amount * new_accrued_token_per_share;
        }
        // // Reset reward debt to 0
        // staker_info.reward_debt = Uint128::zero();
        // Increase length of user reward debt
        staker_info.reward_debt.push(Uint128::zero());
    }
    // Get last reward time
    let last_reward_time = pool_infos.last_reward_time[current_phase_index as usize];
    // Get accrued token per share
    let accrued_token_per_share = pool_infos.accrued_token_per_share[current_phase_index as usize];
    // Get staked token balance from pool contract
    let staked_token_supply = query_token_balance(
        &deps.querier,
        pool_infos.staked_token.clone(),
        env.contract.address,
    )?;

    // update pool
    (new_accrued_token_per_share, new_last_reward_time) = update_pool(
        pool_info.end_time,
        pool_info.reward_per_second,
        staked_token_supply,
        accrued_token_per_share,
        current_time,
        last_reward_time,
    );

    pool_infos.last_reward_time[current_phase_index as usize] = new_last_reward_time;
    pool_infos.accrued_token_per_share[current_phase_index as usize] = new_accrued_token_per_share;

    // Save last pool infos
    POOL_INFOS.save(deps.storage, &pool_infos)?;

    reward_amount += calc_reward_amount(
        staker_info.amount,
        new_accrued_token_per_share,
        staker_info.reward_debt[current_phase_index as usize],
    );

    // Transfer reward token to the sender
    let transfer_reward = match pool_infos.reward_token {
        TokenInfo::Token { contract_addr } => SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount: reward_amount,
            })?,
            funds: vec![],
        })),
        TokenInfo::NativeToken { denom } => SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![coin(reward_amount.into(), denom)],
        })),
    };

    // Withdraw staked token from the pool by using cw20 transfer message
    let withdraw = SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: pool_infos.staked_token,
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: info.sender.to_string(),
            amount,
        })?,
        funds: vec![],
    }));

    // Update staker amount
    staker_info.amount -= amount;
    staker_info.reward_debt[current_phase_index as usize] = staker_info.amount * new_accrued_token_per_share;
    staker_info.joined_phase = current_phase_index;

    // Check if staker amount is zero, remove staker info from storage
    if staker_info.amount == Uint128::zero() {
        STAKERS_INFO.remove(deps.storage, info.sender);
    } else {
        // Update staker info
        STAKERS_INFO.save(deps.storage, info.sender, &staker_info)?;
    }

    res = res
        .add_submessage(transfer_reward)
        .add_submessage(withdraw)
        .add_attribute("previous_receive_reward_time", last_reward_time.to_string())
        .add_attribute("current_time", current_time.to_string())
        .add_attribute("method", "withdraw")
        .add_attribute("withdraw_amount", amount.to_string())
        .add_attribute("reward_amount", reward_amount.to_string());

    Ok(res)
}

// Harvest reward token from the pool to the sender
pub fn execute_harvest(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // Get current time
    let current_time = env.block.time.seconds();
    // Get pool infos
    let mut pool_infos = POOL_INFOS.load(deps.storage)?;
    // Get current pool index
    let current_phase_index = pool_infos.current_phase_index;
    // Get Staker info
    let mut staker_info = STAKERS_INFO
        .may_load(deps.storage, info.sender.clone())?
        .unwrap_or(StakerInfoResponse {
            amount: Uint128::zero(),
            reward_debt: vec![Uint128::zero(); current_phase_index as usize + 1],
            joined_phase: current_phase_index,
        });

    if staker_info.amount == Uint128::zero() {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: Only staker can harvest reward",
        )));
    }

    // Get current pool info in pool infos
    let pool_info = pool_infos.pool_infos[current_phase_index as usize].clone();
    // Init reward amount
    let mut reward_amount = Uint128::zero();
    // get staker joined phases
    let staker_joined_phase = staker_info.joined_phase;
    // Init new accrued token per share
    let mut new_accrued_token_per_share;
    // Init new last reward time
    let mut new_last_reward_time;
    // // Get last reward time
    // let last_reward_time = LAST_REWARD_TIME.load(deps.storage)?;

    // If staker has joined previous phases, loops all pool infos to get reward per second from current pool index to staker joined phases
    if staker_joined_phase < current_phase_index {
        for i in staker_joined_phase..current_phase_index {
            // Get last reward time
            let last_reward_time = pool_infos.last_reward_time[i as usize];
            // Get accrued token per share
            let accrued_token_per_share = pool_infos.accrued_token_per_share[i as usize];
            // Get pool info from pool infos
            let pool_info = POOL_INFOS.load(deps.storage)?.pool_infos[i as usize].clone();
            // Get staked token balance from pool contract previous phases
            let staked_token_supply = pool_info.total_staked_at_end_time;

            // update pool
            (new_accrued_token_per_share, new_last_reward_time) = update_pool(
                pool_info.end_time,
                pool_info.reward_per_second,
                staked_token_supply,
                accrued_token_per_share,
                pool_info.end_time,
                last_reward_time,
            );

            // Update last reward time
            pool_infos.last_reward_time[i as usize] = new_last_reward_time;

            // Save pool infos
            POOL_INFOS.save(deps.storage, &pool_infos)?;

            reward_amount += calc_reward_amount(
                staker_info.amount,
                new_accrued_token_per_share,
                staker_info.reward_debt[i as usize],
            );
            // Update staker reward debt
            staker_info.reward_debt[i as usize] = staker_info.amount * new_accrued_token_per_share;
            // If staker joined phases is less than current phase index reset reward debt to 0
            if staker_info.joined_phase < current_phase_index {
                // staker_info.reward_debt = Uint128::zero();
            }
            // Update staker info
            STAKERS_INFO.save(deps.storage, info.sender.clone(), &staker_info)?;
        }
        // staker_info.reward_debt = Uint128::zero();
        // // Update staker info
        // STAKERS_INFO.save(deps.storage, info.sender.clone(), &staker_info)?;
        // Increase length of user reward debt
        staker_info.reward_debt.push(Uint128::zero());
    }
    // Get last reward time
    let last_reward_time = pool_infos.last_reward_time[current_phase_index as usize];
    // Get accrued token per share
    let accrued_token_per_share = pool_infos.accrued_token_per_share[current_phase_index as usize];

    // Get staked token balance from pool contract
    let staked_token_supply = query_token_balance(
        &deps.querier,
        pool_infos.staked_token.clone(),
        env.contract.address,
    )?;

    // update pool
    (new_accrued_token_per_share, new_last_reward_time) = update_pool(
        pool_info.end_time,
        pool_info.reward_per_second,
        staked_token_supply,
        accrued_token_per_share,
        current_time,
        last_reward_time,
    );

    pool_infos.last_reward_time[current_phase_index as usize] = new_last_reward_time;
    pool_infos.accrued_token_per_share[current_phase_index as usize] = new_accrued_token_per_share;

    // Save pool infos
    POOL_INFOS.save(deps.storage, &pool_infos)?;

    reward_amount += calc_reward_amount(
        staker_info.amount,
        new_accrued_token_per_share,
        staker_info.reward_debt[current_phase_index as usize],
    );
    // Update staker reward debt
    staker_info.reward_debt[current_phase_index as usize] = staker_info.amount * new_accrued_token_per_share;
    // Update staker info
    STAKERS_INFO.save(deps.storage, info.sender.clone(), &staker_info)?;

    // Check if there is any reward to harvest
    if reward_amount == Uint128::zero() {
        return Err(ContractError::Std(StdError::generic_err(
            "InsufficientFunds: Reward amount is zero",
        )));
    }

    // Transfer reward token to the sender
    let transfer = match pool_infos.reward_token {
        TokenInfo::Token { contract_addr } => SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount: reward_amount,
            })?,
            funds: vec![],
        })),
        TokenInfo::NativeToken { denom } => SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![coin(reward_amount.into(), denom)],
        })),
    };

    staker_info.joined_phase = current_phase_index;
    // Update staker info
    STAKERS_INFO.save(deps.storage, info.sender, &staker_info)?;
    let res = Response::new()
        .add_submessage(transfer)
        .add_attribute("previous_receive_reward_time", last_reward_time.to_string())
        .add_attribute("current_time", current_time.to_string())
        .add_attribute("method", "harvest")
        .add_attribute("reward_amount", reward_amount.to_string());

    Ok(res)
}

fn execute_update_pool_limit_per_user(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    new_pool_limit_per_user: Uint128,
) -> Result<Response, ContractError> {
    // Get config
    let config: Config = CONFIG.load(deps.storage)?;
    // Check if the message sender is the owner of the contract
    if config.halo_factory_owner != info.sender {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: Only owner can update pool limit per user",
        )));
    }

    // Get current time
    let current_time = env.block.time.seconds();
    // Get pool infos
    let mut pool_infos = POOL_INFOS.load(deps.storage)?;
    // Get current pool index
    let current_phase_index = pool_infos.current_phase_index;

    // Not allow updating pool limit per user when current time is greater than start time of the phase
    if current_time > pool_infos.pool_infos[current_phase_index as usize].start_time {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: Current time is greater than start time of the phase",
        )));
    }

    // Not allow new pool limit per user is less than previous pool limit per user
    if new_pool_limit_per_user
        < pool_infos.pool_infos[current_phase_index as usize]
            .pool_limit_per_user
            .unwrap_or(Uint128::zero())
    {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: New pool limit per user is less than previous pool limit per user",
        )));
    }

    // Update pool limit per user
    pool_infos.pool_infos[current_phase_index as usize].pool_limit_per_user =
        Some(new_pool_limit_per_user);
    // Save pool info
    POOL_INFOS.save(deps.storage, &pool_infos)?;

    let res = Response::new()
        .add_attribute("method", "update_pool_limit_per_user")
        .add_attribute(
            "new_pool_limit_per_user",
            new_pool_limit_per_user.to_string(),
        );

    Ok(res)
}

pub fn execute_add_phase(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    new_start_time: u64,
    new_end_time: u64,
) -> Result<Response, ContractError> {
    // Get current time
    let current_time = env.block.time.seconds();
    // Not allow new start time is greater than new end time
    if new_start_time >= new_end_time {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: New start time is greater than new end time",
        )));
    }
    // Not allow to add new phase when current time is greater than new start time
    if current_time > new_start_time {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: Current time is greater than new start time",
        )));
    }
    // Get config
    let config: Config = CONFIG.load(deps.storage)?;
    // Check if the message sender is the owner of the contract
    if config.halo_factory_owner != info.sender {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: Only owner can add new phase",
        )));
    }
    // Get pool infos
    let mut pool_infos: PoolInfos = POOL_INFOS.load(deps.storage)?;
    // Get current pool infos length
    let pool_length = pool_infos.pool_infos.len();
    // Get current phase index
    let current_phase_index = pool_infos.current_phase_index;

    // Not allow add new phase when previous phase is not active yet
    if pool_length as u64 > current_phase_index + 1 {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: Previous phase is not active",
        )));
    }

    // Get phases reward balance
    let mut phases_reward_balance = pool_infos.reward_balance;
    // Get last reward time
    let mut phases_last_reward_time = pool_infos.last_reward_time;
    // Get accrued token per share
    let mut phases_accrued_token_per_share = pool_infos.accrued_token_per_share;
    // Increase length of pool infos
    pool_infos.pool_infos.push(PoolInfo {
        reward_per_second: Decimal::zero(),
        start_time: new_start_time,
        end_time: new_end_time,
        pool_limit_per_user: pool_infos.pool_infos[pool_infos.current_phase_index as usize]
            .pool_limit_per_user,
        total_staked_at_end_time: Uint128::zero(),
    });

    // Increase length of reward balance
    phases_reward_balance.push(Uint128::zero());
    // Increase length of last reward time
    phases_last_reward_time.push(new_start_time);
    // Increase length of accrued token per share
    phases_accrued_token_per_share.push(Decimal::zero());

    // Update pool infos
    pool_infos.reward_balance = phases_reward_balance;
    pool_infos.last_reward_time = phases_last_reward_time;
    pool_infos.accrued_token_per_share = phases_accrued_token_per_share;

    // Save pool infos
    POOL_INFOS.save(deps.storage, &pool_infos)?;

    let res = Response::new()
        .add_attribute("method", "add_phase")
        .add_attribute("new_start_time", new_start_time.to_string())
        .add_attribute("new_end_time", new_end_time.to_string());

    Ok(res)
}

pub fn execute_activate_phase(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // Get current time
    let current_time = env.block.time.seconds();
    // Get config
    let config: Config = CONFIG.load(deps.storage)?;

    // Check if the message sender is the owner of the contract
    if config.halo_factory_owner != info.sender {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: Only owner can active new phase",
        )));
    }
    // Get pool infos
    let mut pool_infos: PoolInfos = POOL_INFOS.load(deps.storage)?;

    // Not allow activating phase when current time is less than end time of the current pool
    // or greater than start time of the phase to be activated
    if current_time
        < pool_infos.pool_infos[pool_infos.current_phase_index as usize].end_time
        || current_time
            > pool_infos.pool_infos[pool_infos.current_phase_index as usize + 1].start_time
    {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: Current time is not in range of the phase to be activated",
        )));
    }

    // query total staked token
    let total_staked_token = query_token_balance(
        &deps.querier,
        pool_infos.staked_token.clone(),
        env.contract.address.clone(),
    )?;

    // Update total staked token
    pool_infos.pool_infos[pool_infos.current_phase_index as usize].total_staked_at_end_time =
        total_staked_token;

    // Increase current pool index
    pool_infos.current_phase_index += 1;

    // Save pool infos
    POOL_INFOS.save(deps.storage, &pool_infos)?;

    Ok(Response::new().add_attributes([
        ("method", "activate_phase"),
        (
            "activated_phase",
            &pool_infos.current_phase_index.to_string(),
        ),
    ]))
}

pub fn query_token_balance(
    querier: &QuerierWrapper,
    contract_addr: String,
    account_addr: Addr,
) -> StdResult<Uint128> {
    let res: Cw20BalanceResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr,
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
        QueryMsg::PendingReward { address } => {
            Ok(to_binary(&query_pending_reward(deps, env, address)?)?)
        }
        QueryMsg::TotalStaked {} => Ok(to_binary(&query_total_lp_token_staked(deps, env)?)?),
        QueryMsg::StakerInfo { address } => Ok(to_binary(&query_staker_info(deps, address)?)?),
    }
}

fn query_pool_info(deps: Deps) -> Result<PoolInfos, ContractError> {
    Ok(POOL_INFOS.load(deps.storage)?)
}

fn query_pending_reward(
    deps: Deps,
    env: Env,
    address: String,
) -> Result<RewardTokenAssetResponse, ContractError> {
    // Get current time
    let current_time = env.block.time.seconds();
    // Get pool infos
    let pool_infos = POOL_INFOS.load(deps.storage)?;
    // Check if staker has staked in the pool
    if STAKERS_INFO.may_load(deps.storage, Addr::unchecked(address.clone()))?.is_none() {
        return Ok(RewardTokenAssetResponse {
            info: pool_infos.reward_token,
            amount: Uint128::zero(),
            time_query: current_time,
        });
    }
    // Get current pool index
    let current_phase_index = pool_infos.current_phase_index;
    // Get current pool info in pool infos
    let pool_info = POOL_INFOS.load(deps.storage)?.pool_infos[current_phase_index as usize].clone();
    // Get last reward time
    let last_reward_time = pool_infos.last_reward_time;
    // Get accrued token per share
    let mut accrued_token_per_share = pool_infos.accrued_token_per_share;
    // Multiply factor
    let mut multiplier: u64;

    // Get staker info
    let mut staker_info = STAKERS_INFO
        .load(deps.storage, Addr::unchecked(address)).unwrap();

    // get staker joined phases
    let staker_joined_phase = staker_info.joined_phase;
    // Init reward amount
    let mut reward_amount = Uint128::zero();

    if staker_joined_phase < current_phase_index {
        for i in staker_joined_phase..current_phase_index {
            // Get pool info from pool infos
            let pool_info = POOL_INFOS.load(deps.storage)?.pool_infos[i as usize].clone();
            // Get staked token balance from pool contract previous phases
            let staked_token_supply = pool_info.total_staked_at_end_time;

            multiplier = get_multiplier(
                last_reward_time[i as usize],
                pool_info.end_time,
                pool_info.end_time,
            );

            let reward = Decimal::new(multiplier.into()) * pool_info.reward_per_second;
            accrued_token_per_share[i as usize] += reward / Decimal::new(staked_token_supply);
            reward_amount += calc_reward_amount(
                staker_info.amount,
                accrued_token_per_share[i as usize],
                staker_info.reward_debt[i as usize],
            );
            println!("reward_amount: {}", reward_amount);
        }
        // staker_info.reward_debt = Uint128::zero();
    }

    // If phase not started yet
    if current_time < pool_info.start_time {
        multiplier = 0u64;
    } else {
        multiplier = get_multiplier(
            last_reward_time[current_phase_index as usize],
            current_time,
            pool_info.end_time,
        );
    }

    // Get staked token balance from pool contract
    let staked_token_supply = query_token_balance(
        &deps.querier,
        pool_infos.staked_token.clone(),
        env.contract.address,
    )?;

    let reward: Decimal = Decimal::new(multiplier.into()) * pool_info.reward_per_second;

    accrued_token_per_share[current_phase_index as usize] +=
        reward / Decimal::new(staked_token_supply);

    // Init new reward debt for current phase
    let mut reward_debt = Uint128::zero();

    // If staker has joined current phase update reward debt
    if staker_info.joined_phase == current_phase_index {
        reward_debt = staker_info.reward_debt[current_phase_index as usize];
    }

    reward_amount += calc_reward_amount(
        staker_info.amount,
        accrued_token_per_share[current_phase_index as usize],
        reward_debt,
    );

    Ok(RewardTokenAssetResponse {
        info: pool_infos.reward_token,
        amount: reward_amount,
        time_query: current_time,
    })
}

fn query_total_lp_token_staked(deps: Deps, env: Env) -> Result<Uint128, ContractError> {
    // Get pool infos
    let pool_infos = POOL_INFOS.load(deps.storage)?;
    let staked_token_supply =
        query_token_balance(&deps.querier, pool_infos.staked_token, env.contract.address)?;
    Ok(staked_token_supply)
}

fn query_staker_info(deps: Deps, address: String) -> Result<StakerInfoResponse, ContractError> {
    // Get staker info
    let staker_info = STAKERS_INFO
        .may_load(deps.storage, Addr::unchecked(address))?
        .unwrap_or(StakerInfoResponse {
            amount: Uint128::zero(),
            reward_debt: vec![Uint128::zero()],
            joined_phase: 0u64,
        });
    Ok(staker_info)
}
