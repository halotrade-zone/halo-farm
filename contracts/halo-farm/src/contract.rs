use std::env;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, has_coins, to_binary, Addr, BalanceResponse, BankMsg, BankQuery, Binary, Coin, CosmosMsg,
    Decimal, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, QueryRequest, Response, StdError,
    StdResult, SubMsg, Uint128, WasmMsg, WasmQuery,
};

use cw2::set_contract_version;
use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:halo-farm";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

use crate::{
    error::ContractError,
    formulas::calc_reward_amount,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::{
        Config, FarmInfo, PendingRewardResponse, PhaseInfo, StakerInfo, StakerInfoResponse,
        TokenInfo, CONFIG, FARM_INFO, STAKERS_INFO,
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

pub fn execute_add_reward_balance(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    phase_index: u64,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // Get farm info
    let mut farm_info = FARM_INFO.load(deps.storage)?;
    // Get current phase index
    let current_phase_index = farm_info.current_phase_index;

    // Not allow to add reward balance to activated phase
    if phase_index <= current_phase_index && current_phase_index != 0 {
        return Err(ContractError::Std(StdError::generic_err(
            "Can not add reward balance to activated phase",
        )));
    }

    // Check the message sender is the whitelisted address
    if farm_info.phases_info[phase_index as usize].whitelist != info.sender {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: Sender is not whitelisted address",
        )));
    }

    // Get phase info in farm info
    let phase_info = farm_info.phases_info[phase_index as usize].clone();
    // Init response
    let mut res = Response::new();
    // Verify reward token asset
    // Add reward balance to the phase
    // When creating a new farm or adding a new phase, sender must add balance amount of reward_token
    // Match reward token type:
    // 1. If reward token is native token, sender must add balance amount of native token
    //    to the new farm contract address by sending via funds when calling this msg.
    // 2. If reward token is cw20 token, sender must add balance amount of cw20 token
    //    to the new farm contract address by calling cw20 contract transfer_from method.
    match farm_info.reward_token.clone() {
        TokenInfo::Token { contract_addr } => {
            let transfer = SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: info.sender.to_string(),
                    recipient: env.contract.address.to_string(),
                    amount,
                })?,
                funds: vec![],
            }));
            res = res.add_submessage(transfer);
        }
        TokenInfo::NativeToken { denom } => {
            // If reward token is native token, check the denom and amount of asset is valid
            if !has_coins(&info.funds, &Coin { denom, amount }) {
                return Err(ContractError::Std(StdError::generic_err(
                    "Native token balance mismatch between the argument and the transferred",
                )));
            }
        }
    }

    // Get current time
    let current_time = env.block.time.seconds();

    // Not allow adding reward balance when current time is greater than start time of the phase
    if current_time > phase_info.start_time {
        return Err(ContractError::Std(StdError::generic_err(
            "Current time is greater than start time of the phase",
        )));
    }

    // Get the reward token balance of a phase in multiple phases.
    let mut reward_balance = farm_info.phases_info[phase_index as usize].reward_balance;

    // Update reward balance
    reward_balance += amount;

    let new_phase_info = PhaseInfo {
        reward_balance,
        ..phase_info
    };

    // Save phase info to farm info in current phase index
    farm_info.phases_info[phase_index as usize] = new_phase_info;
    FARM_INFO.save(deps.storage, &farm_info)?;

    Ok(res
        .add_attribute("method", "add_reward_balance")
        .add_attribute("sender", info.sender)
        .add_attribute("phase_index", phase_index.to_string())
        .add_attribute("reward_token_asset", farm_info.reward_token.to_string())
        .add_attribute("amount", amount.to_string()))
}

// pub fn execute_remove_reward_balance(
//     deps: DepsMut,
//     env: Env,
//     info: MessageInfo,
//     phase_index: u64,
// ) -> Result<Response, ContractError> {
//     let current_time = env.block.time;
//     // Get phase info in farm info
//     let phase_info = FARM_INFO.load(deps.storage)?.phases_info[phase_index as usize].clone();
//     let reward_token_balance: Uint128;
//     // Check the message sender is the whitelisted address
//     if !phase_info.whitelist.contains(&info.sender) {
//         return Err(ContractError::Unauthorized {});
//     }

//     // Only can remove reward balance when the phase is inactive
//     if current_time.seconds() < phase_info.start_time || current_time.seconds() > phase_info.end_time {
//         return Err(ContractError::Std(StdError::generic_err(
//             "Can not remove reward balance when the phase is active",
//         )));
//     }

//     // Transfer all remaining reward token balance to the sender
//     let transfer_reward = match phase_info.reward_token {
//         TokenInfo::Token { contract_addr } => {
//             // Query reward token balance of the phase
//             reward_token_balance = query_token_balance(&deps.querier, contract_addr.clone(), env.contract.address.clone())?;
//             // Check if the reward token balance of the phase is greater than 0
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
//             // Query reward token balance of the phase
//             reward_token_balance = query_balance(&deps.querier, env.contract.address.clone(), denom.clone())?;
//             // Check if the reward token balance of the phase is greater than 0
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
    // Get farm info
    let mut farm_info = FARM_INFO.load(deps.storage)?;
    // Get current phase index
    let current_phase_index = farm_info.current_phase_index;

    // Not allow removing activated phase
    if phase_index <= current_phase_index {
        return Err(ContractError::Std(StdError::generic_err(
            "Can not remove activated phase",
        )));
    }

    // Get config
    let config: Config = CONFIG.load(deps.storage)?;

    // Check if the message sender is the owner of the contract
    if config.farm_owner != info.sender {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: Only owner can remove phase",
        )));
    }

    // Init response
    let mut res = Response::new();
    // If phase already added reward balance, transfer back all phase reward balance to the sender
    if farm_info.phases_info[phase_index as usize].reward_balance > Uint128::zero() {
        // Get whitelist address
        let whitelist = farm_info.phases_info[phase_index as usize]
            .whitelist
            .clone();
        // Transfer reward balance to whitelist address
        let transfer_reward = match farm_info.reward_token.clone() {
            TokenInfo::Token { contract_addr } => SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: whitelist.to_string(),
                    amount: farm_info.phases_info[phase_index as usize].reward_balance,
                })?,
                funds: vec![],
            })),
            TokenInfo::NativeToken { denom } => SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: whitelist.to_string(),
                amount: vec![coin(
                    farm_info.phases_info[phase_index as usize]
                        .reward_balance
                        .into(),
                    denom,
                )],
            })),
        };
        res = res.add_submessage(transfer_reward).add_attribute(
            "transfer_reward",
            farm_info.phases_info[phase_index as usize]
                .reward_balance
                .to_string(),
        );
    }
    // Remove phase
    farm_info.phases_info.remove(phase_index as usize);
    // Save farm info
    FARM_INFO.save(deps.storage, &farm_info)?;

    Ok(res
        .add_attribute("method", "remove_phase")
        .add_attribute("phase_index", phase_index.to_string()))
}

fn claim_all_reward(
    farm_info: &mut FarmInfo,
    staker_info: &mut StakerInfo,
    current_time: u64,
) -> (Uint128, Decimal) {
    let mut reward_amount = Uint128::zero();
    let current_phase_index = farm_info.current_phase_index;

    // If staker has joined previous phases, loops all farm info to get reward per second from current phase index to staker joined phases
    for i in staker_info.joined_phase..current_phase_index {
        // Get accrued token per share
        let accrued_token_per_share = farm_info.phases_info[i as usize].accrued_token_per_share;

        // Calculate reward amount
        reward_amount += calc_reward_amount(
            staker_info.amount,
            accrued_token_per_share,
            staker_info.reward_debt[i as usize],
        );
        // Update staker info
        staker_info.reward_debt[i as usize] = staker_info.amount * accrued_token_per_share;
        // Increase length of user reward debt to current phase index
        staker_info.reward_debt.push(Uint128::zero());
    }

    let mut phase_info = farm_info.phases_info[current_phase_index as usize].clone();
    let staked_token_balance = farm_info.staked_token_balance;

    let (new_accrued_token_per_share, new_last_reward_time) =
        phase_info.update_reward_ratio_and_time(current_time, staked_token_balance);

    phase_info.last_reward_time = new_last_reward_time;
    phase_info.accrued_token_per_share = new_accrued_token_per_share;

    farm_info.phases_info[current_phase_index as usize] = phase_info;
    reward_amount += calc_reward_amount(
        staker_info.amount,
        new_accrued_token_per_share,
        staker_info.reward_debt[current_phase_index as usize],
    );

    (reward_amount, new_accrued_token_per_share)
}

pub fn execute_deposit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // Not allow depositing 0 amount
    if amount.is_zero() {
        return Err(ContractError::Std(StdError::generic_err(
            "InvalidZeroAmount: Deposit amount is 0",
        )));
    }
    // Get farm info
    let mut farm_info = FARM_INFO.load(deps.storage)?;
    // Get current phase index
    let current_phase_index = farm_info.current_phase_index;
    // Get current phase info in farm info
    let phase_info = farm_info.phases_info[current_phase_index as usize].clone();
    // Not allow depositing if reward token is not added to the phase yet
    if phase_info.reward_balance == Uint128::zero() {
        return Err(ContractError::Std(StdError::generic_err("Empty phase")));
    }
    // If staker has not joined any phase, save initial staker info
    if STAKERS_INFO
        .may_load(deps.storage, info.sender.clone())?
        .is_none()
    {
        STAKERS_INFO.save(
            deps.storage,
            info.sender.clone(),
            &StakerInfo {
                amount: Uint128::zero(),
                reward_debt: vec![Uint128::zero(); current_phase_index as usize + 1],
                joined_phase: current_phase_index,
            },
        )?;
    }

    // Get current time
    let current_time = env.block.time.seconds();
    // Not allow depositing when current time is greater than end time of the phase
    if current_time > phase_info.end_time {
        return Err(ContractError::Std(StdError::generic_err(
            "Current time is not in the range of the phase",
        )));
    }

    // Get staker info
    let mut staker_info = STAKERS_INFO.load(deps.storage, info.sender.clone())?;

    // Check phase limit per user
    if let Some(phases_limit_per_user) = farm_info.phases_limit_per_user {
        if staker_info.amount + amount > phases_limit_per_user {
            return Err(ContractError::Std(StdError::generic_err(
                "Deposit amount exceeds phase limit per user",
            )));
        }
    }

    // Init response
    let mut res = Response::new();
    let (reward_amount, new_accrued_token_per_share) =
        claim_all_reward(&mut farm_info, &mut staker_info, current_time);

    // If reward amount is greater than 0, transfer reward amount to staker
    if reward_amount > Uint128::zero() {
        let transfer_reward = match farm_info.reward_token.clone() {
            TokenInfo::Token { contract_addr } => SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
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

    // Deposit staked token to the farm contract
    let transfer = SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: farm_info.staked_token.to_string(),
        msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
            owner: info.sender.to_string(),
            recipient: env.contract.address.to_string(),
            amount,
        })?,
        funds: vec![],
    }));

    // Increase staked token balance
    farm_info.staked_token_balance += amount;

    // Update staker info
    staker_info.amount += amount;
    staker_info.reward_debt[current_phase_index as usize] =
        staker_info.amount * new_accrued_token_per_share;
    staker_info.joined_phase = current_phase_index;

    STAKERS_INFO.save(deps.storage, info.sender, &staker_info)?;
    // Save farm info
    FARM_INFO.save(deps.storage, &farm_info)?;

    res = res
        .add_submessage(transfer)
        .add_attribute("current_time", current_time.to_string())
        .add_attribute("method", "deposit")
        .add_attribute("deposit_amount", amount.to_string())
        .add_attribute("harvest_reward_amount", reward_amount.to_string());

    Ok(res)
}

pub fn execute_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // Not allow withdrawing 0 amount
    if amount.is_zero() {
        return Err(ContractError::Std(StdError::generic_err(
            "InvalidZeroAmount: Withdraw amount is 0",
        )));
    }
    // Get farm info
    let mut farm_info = FARM_INFO.load(deps.storage)?;
    // Get current phase index
    let current_phase_index = farm_info.current_phase_index;
    // Only staker can withdraw
    if STAKERS_INFO
        .may_load(deps.storage, info.sender.clone())?
        .is_none()
    {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: Only staker can withdraw",
        )));
    }

    // Get staker info
    let mut staker_info = STAKERS_INFO.load(deps.storage, info.sender.clone())?;

    // Check staker amount is greater than withdraw amount
    if staker_info.amount < amount {
        return Err(ContractError::Std(StdError::generic_err(
            "InsufficientFunds: Withdraw amount exceeds staked amount",
        )));
    }

    // Init response
    let mut res = Response::new();
    // Get current time
    let current_time = env.block.time.seconds();

    // Get all reward info
    let (reward_amount, new_accrued_token_per_share) =
        claim_all_reward(&mut farm_info, &mut staker_info, current_time);

    // If reward amount is greater than 0, transfer reward token to the sender
    if reward_amount > Uint128::zero() {
        let transfer_reward = match farm_info.reward_token.clone() {
            TokenInfo::Token { contract_addr } => SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
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

    // Withdraw staked token from the farm contract by using cw20 transfer message
    let withdraw = SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: farm_info.staked_token.to_string(),
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: info.sender.to_string(),
            amount,
        })?,
        funds: vec![],
    }));

    // Decrease staked token balance
    farm_info.staked_token_balance -= amount;

    // Update staker amount
    staker_info.amount -= amount;
    staker_info.reward_debt[current_phase_index as usize] =
        staker_info.amount * new_accrued_token_per_share;
    staker_info.joined_phase = current_phase_index;

    // Check if staker amount is zero, remove staker info from storage
    if staker_info.amount == Uint128::zero() {
        STAKERS_INFO.remove(deps.storage, info.sender);
    } else {
        // Update staker info
        STAKERS_INFO.save(deps.storage, info.sender, &staker_info)?;
    }
    // Save farm info
    FARM_INFO.save(deps.storage, &farm_info)?;

    res = res
        .add_submessage(withdraw)
        .add_attribute("current_time", current_time.to_string())
        .add_attribute("method", "withdraw")
        .add_attribute("withdraw_amount", amount.to_string())
        .add_attribute("harvest_reward_amount", reward_amount.to_string());

    Ok(res)
}

// Harvest reward token from the farm contract to the sender
pub fn execute_harvest(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // Only staker can harvest reward
    if STAKERS_INFO
        .may_load(deps.storage, info.sender.clone())?
        .is_none()
    {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: Only staker can harvest reward",
        )));
    }

    // Get current time
    let current_time = env.block.time.seconds();
    // Get farm info
    let mut farm_info = FARM_INFO.load(deps.storage)?;
    // Get current phase index
    let current_phase_index = farm_info.current_phase_index;
    // Get staker info
    let mut staker_info = STAKERS_INFO.load(deps.storage, info.sender.clone())?;

    // Get all reward info
    let (reward_amount, new_accrued_token_per_share) =
        claim_all_reward(&mut farm_info, &mut staker_info, current_time);

    // Update staker reward debt
    staker_info.reward_debt[current_phase_index as usize] =
        staker_info.amount * new_accrued_token_per_share;
    // Update staker info
    STAKERS_INFO.save(deps.storage, info.sender.clone(), &staker_info)?;
    // Save farm info
    FARM_INFO.save(deps.storage, &farm_info)?;

    // Check if there is any reward to harvest
    if reward_amount == Uint128::zero() {
        return Err(ContractError::Std(StdError::generic_err(
            "InsufficientFunds: Reward amount is zero",
        )));
    }

    // Transfer reward token to the sender
    let transfer = match farm_info.reward_token {
        TokenInfo::Token { contract_addr } => SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract_addr.to_string(),
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
        .add_attribute("current_time", current_time.to_string())
        .add_attribute("method", "harvest")
        .add_attribute("reward_amount", reward_amount.to_string());

    Ok(res)
}

// fn execute_update_phases_limit_per_user(
//     deps: DepsMut,
//     env: Env,
//     info: MessageInfo,
//     new_phases_limit_per_user: Uint128,
// ) -> Result<Response, ContractError> {
//     // Get config
//     let config: Config = CONFIG.load(deps.storage)?;
//     // Check if the message sender is the owner of the contract
//     if config.farm_owner != info.sender {
//         return Err(ContractError::Std(StdError::generic_err(
//             "Unauthorized: Only owner can update phases limit per user",
//         )));
//     }

//     // Get current time
//     let current_time = env.block.time.seconds();
//     // Get farm info
//     let mut phases_info = FARM_INFO.load(deps.storage)?;
//     // Get current phase index
//     let current_phase_index = phases_info.current_phase_index;

//     // Not allow updating phases limit per user when current time is greater than start time of the phase
//     if current_time > phases_info.phases_info[current_phase_index as usize].start_time {
//         return Err(ContractError::Std(StdError::generic_err(
//             "Current time is greater than start time of the phase",
//         )));
//     }

//     // Not allow new phases limit per user is less than previous phases limit per user
//     if new_phases_limit_per_user
//         < phases_info.phases_info[current_phase_index as usize]
//             .phases_limit_per_user
//             .unwrap_or(Uint128::zero())
//     {
//         return Err(ContractError::Std(StdError::generic_err(
//             "New phases limit per user is less than previous phases limit per user",
//         )));
//     }

//     // Update phases limit per user
//     phases_info.phases_info[current_phase_index as usize].phases_limit_per_user =
//         Some(new_phases_limit_per_user);
//     // Save farm info
//     FARM_INFO.save(deps.storage, &phases_info)?;

//     let res = Response::new()
//         .add_attribute("method", "update_phases_limit_per_user")
//         .add_attribute(
//             "new_phases_limit_per_user",
//             new_phases_limit_per_user.to_string(),
//         );

//     Ok(res)
// }

pub fn execute_add_phase(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    new_start_time: u64,
    new_end_time: u64,
    whitelist: Addr,
) -> Result<Response, ContractError> {
    // Get config
    let config: Config = CONFIG.load(deps.storage)?;

    // Check if the message sender is the owner of the contract
    if config.farm_owner != info.sender {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: Only owner can add new phase",
        )));
    }
    // Not allow new start time is greater than new end time
    if new_start_time >= new_end_time {
        return Err(ContractError::Std(StdError::generic_err(
            "New start time is greater than new end time",
        )));
    }
    // Get current time
    let current_time = env.block.time.seconds();
    // Not allow to add new phase when current time is greater than new start time
    if current_time > new_start_time {
        return Err(ContractError::Std(StdError::generic_err(
            "Current time is greater than new start time",
        )));
    }

    // Get farm info
    let mut farm_info: FarmInfo = FARM_INFO.load(deps.storage)?;
    // Get current farm info length
    let phases_length = farm_info.phases_info.len();
    // Get current phase index
    let current_phase_index = farm_info.current_phase_index;

    // Not allow add new phase when new start time is less than end time of the current phase
    if new_start_time < farm_info.phases_info[current_phase_index as usize].end_time {
        return Err(ContractError::Std(StdError::generic_err(
            "New start time is less than end time of the current phase",
        )));
    }

    // Not allow add new phase when previous phase is not active yet
    if phases_length as u64 - 1 > current_phase_index {
        return Err(ContractError::Std(StdError::generic_err(
            "Previous phase is not active",
        )));
    }

    // Increase length of farm info
    farm_info.phases_info.push(PhaseInfo {
        start_time: new_start_time,
        end_time: new_end_time,
        whitelist: whitelist.clone(),
        reward_balance: Uint128::zero(),
        last_reward_time: new_start_time,
        accrued_token_per_share: Decimal::zero(),
    });

    // Save farm info
    FARM_INFO.save(deps.storage, &farm_info)?;

    let res = Response::new()
        .add_attribute("method", "add_phase")
        .add_attribute("new_start_time", new_start_time.to_string())
        .add_attribute("new_end_time", new_end_time.to_string())
        .add_attribute("whitelist", whitelist.to_string());

    Ok(res)
}

pub fn execute_activate_phase(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // Get config
    let config: Config = CONFIG.load(deps.storage)?;

    // Check if the message sender is the owner of the contract
    if config.farm_owner != info.sender {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: Only owner can active new phase",
        )));
    }
    // Get farm info
    let farm_info: FarmInfo = FARM_INFO.load(deps.storage)?;
    // Get current phase index
    let current_phase_index = farm_info.current_phase_index as usize;

    // Not allow active phase when current phase index is equal to farm info length
    // If sender want to active new phase, they have to add new phase first
    if farm_info.phases_info.len() == current_phase_index {
        return Err(ContractError::Std(StdError::generic_err(
            "Phase is already activated",
        )));
    }

    // Get current time
    let current_time = env.block.time.seconds();

    // Not allow activating phase when current time is less than end time of the current phase
    // or greater than start time of the phase to be activated
    if current_time < farm_info.phases_info[current_phase_index].end_time
        || current_time > farm_info.phases_info[current_phase_index + 1].start_time
    {
        return Err(ContractError::Std(StdError::generic_err(
            "Current time is not in range of the phase to be activated",
        )));
    }

    // Not allow activating phase when reward balance of this phase is zero
    if farm_info.phases_info[current_phase_index + 1]
        .reward_balance
        .is_zero()
    {
        return Err(ContractError::Std(StdError::generic_err("Empty phase")));
    }

    // Get staked token balance
    let staked_token_balance = farm_info.staked_token_balance;
    // Get farm info
    let mut farm_info: FarmInfo = FARM_INFO.load(deps.storage)?;

    // Get phase info from farm info
    let phase_info = farm_info.phases_info[current_phase_index].clone();

    // get new reward ratio and time
    let (new_accrued_token_per_share, _new_last_reward_time) =
        phase_info.update_reward_ratio_and_time(phase_info.end_time, staked_token_balance);

    farm_info.phases_info[current_phase_index].last_reward_time = phase_info.end_time;
    farm_info.phases_info[current_phase_index].accrued_token_per_share =
        new_accrued_token_per_share;

    // Increase current phase index to activate new phase
    farm_info.current_phase_index += 1;

    // Save farm info
    FARM_INFO.save(deps.storage, &farm_info)?;

    Ok(Response::new().add_attributes([
        ("method", "activate_phase"),
        (
            "activated_phase",
            &farm_info.current_phase_index.to_string(),
        ),
    ]))
}

pub fn query_token_balance(
    querier: &QuerierWrapper,
    contract_addr: Addr,
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

fn query_farm_info(deps: Deps) -> StdResult<FarmInfo> {
    FARM_INFO.load(deps.storage)
}

fn query_pending_reward(deps: Deps, env: Env, address: String) -> StdResult<PendingRewardResponse> {
    // Get current time
    let current_time = env.block.time.seconds();
    // Get farm info
    let mut farm_info = FARM_INFO.load(deps.storage)?;
    // Check if staker has staked in the farm contract
    if STAKERS_INFO
        .may_load(deps.storage, Addr::unchecked(address.clone()))?
        .is_none()
    {
        return Ok(PendingRewardResponse {
            info: farm_info.reward_token,
            amount: Uint128::zero(),
            time_query: current_time,
        });
    }
    // Get staker info
    let mut staker_info = STAKERS_INFO
        .load(deps.storage, Addr::unchecked(address))
        .unwrap();

    let (reward_amount, _new_accrued_token_per_share) =
        claim_all_reward(&mut farm_info, &mut staker_info, current_time);

    Ok(PendingRewardResponse {
        info: farm_info.reward_token,
        amount: reward_amount,
        time_query: current_time,
    })
}

fn query_total_lp_token_staked(deps: Deps) -> StdResult<Uint128> {
    Ok(FARM_INFO.load(deps.storage)?.staked_token_balance)
}

fn query_staker_info(deps: Deps, address: String) -> StdResult<StakerInfoResponse> {
    // Get staker info
    let staker_info = STAKERS_INFO
        .may_load(deps.storage, Addr::unchecked(address))?
        .unwrap_or(StakerInfo {
            amount: Uint128::zero(),
            reward_debt: vec![],
            joined_phase: 0u64,
        });
    Ok(StakerInfoResponse {
        amount: staker_info.amount,
        joined_phase: staker_info.joined_phase,
    })
}
