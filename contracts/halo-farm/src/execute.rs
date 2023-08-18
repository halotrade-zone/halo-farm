use crate::{
    error::ContractError,
    formulas::calc_reward_amount,
    state::{Config, FarmInfo, PhaseInfo, StakerInfo, TokenInfo, CONFIG, FARM_INFO, STAKERS_INFO},
};
use cosmwasm_std::{
    coins, has_coins, wasm_execute, Addr, BankMsg, Coin, CosmosMsg, Decimal, DepsMut, Env,
    MessageInfo, Response, StdError, Uint128,
};
use cw20::Cw20ExecuteMsg;

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
        return Err(ContractError::Std(StdError::generic_err("Phase activated")));
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
            let transfer = wasm_execute(
                contract_addr.to_string(),
                &Cw20ExecuteMsg::TransferFrom {
                    owner: info.sender.to_string(),
                    recipient: env.contract.address.to_string(),
                    amount,
                },
                vec![],
            )?;
            res = res.add_message(transfer);
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
        match farm_info.reward_token.clone() {
            TokenInfo::Token { contract_addr } => {
                let transfer_reward = wasm_execute(
                    contract_addr.to_string(),
                    &Cw20ExecuteMsg::Transfer {
                        recipient: whitelist.to_string(),
                        amount: farm_info.phases_info[phase_index as usize].reward_balance,
                    },
                    vec![],
                )?;
                res = res.add_message(transfer_reward);
            }
            TokenInfo::NativeToken { denom } => {
                res = res.add_message(CosmosMsg::Bank(BankMsg::Send {
                    to_address: whitelist.to_string(),
                    amount: coins(
                        farm_info.phases_info[phase_index as usize]
                            .reward_balance
                            .into(),
                        denom,
                    ),
                }))
            }
        };
        res = res.add_attribute(
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

pub fn claim_all_reward(
    farm_info: &mut FarmInfo,
    staker_info: &mut StakerInfo,
    current_time: u64,
) -> Uint128 {
    let mut reward_amount = Uint128::zero();
    let &current_phase_index = &farm_info.current_phase_index;

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

    let phase_info = &mut farm_info.phases_info[current_phase_index as usize];
    let staked_token_balance = farm_info.staked_token_balance;

    phase_info.update_reward_ratio_and_time(current_time, staked_token_balance);

    reward_amount += calc_reward_amount(
        staker_info.amount,
        phase_info.accrued_token_per_share,
        staker_info.reward_debt[current_phase_index as usize],
    );

    reward_amount
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

    let farm_info = &mut FARM_INFO.load(deps.storage)?;
    let current_phase_index: usize = farm_info.current_phase_index as usize;

    // Not allow depositing if reward token is not added to the phase yet
    if farm_info.phases_info[current_phase_index].reward_balance == Uint128::zero() {
        return Err(ContractError::Std(StdError::generic_err("Empty phase")));
    }

    let mut staker_info = STAKERS_INFO
        .load(deps.storage, info.sender.clone())
        .unwrap_or(StakerInfo {
            amount: Uint128::zero(),
            reward_debt: vec![Uint128::zero(); current_phase_index + 1],
            joined_phase: current_phase_index as u64,
        });

    let current_time = env.block.time.seconds();
    // Not allow depositing when current time is greater than end time of the phase
    if current_time > farm_info.phases_info[current_phase_index].end_time {
        return Err(ContractError::Std(StdError::generic_err(
            "Current time is not in the range of the phase",
        )));
    }

    // Check phase limit per user
    if let Some(phases_limit_per_user) = farm_info.phases_limit_per_user {
        if staker_info.amount + amount > phases_limit_per_user {
            return Err(ContractError::Std(StdError::generic_err(
                "Deposit amount exceeds phase limit per user",
            )));
        }
    }

    let mut res = Response::new();
    let reward_amount = claim_all_reward(farm_info, &mut staker_info, current_time);

    // If reward amount is greater than 0, transfer reward amount to staker
    if reward_amount > Uint128::zero() {
        match farm_info.reward_token.clone() {
            TokenInfo::Token { contract_addr } => {
                let transfer_reward = wasm_execute(
                    contract_addr.to_string(),
                    &Cw20ExecuteMsg::Transfer {
                        recipient: info.sender.to_string(),
                        amount: reward_amount,
                    },
                    vec![],
                )?;
                res = res.add_message(transfer_reward);
            }
            TokenInfo::NativeToken { denom } => {
                res = res.add_message(CosmosMsg::Bank(BankMsg::Send {
                    to_address: info.sender.to_string(),
                    amount: coins(reward_amount.into(), denom),
                }))
            }
        };
    }

    // Deposit staked token to the farm contract
    let transfer = wasm_execute(
        farm_info.staked_token.to_string(),
        &Cw20ExecuteMsg::TransferFrom {
            owner: info.sender.to_string(),
            recipient: env.contract.address.to_string(),
            amount,
        },
        vec![],
    )?;

    farm_info.staked_token_balance += amount;

    staker_info.amount += amount;
    staker_info.reward_debt[current_phase_index] =
        staker_info.amount * farm_info.phases_info[current_phase_index].accrued_token_per_share;
    staker_info.joined_phase = current_phase_index as u64;

    FARM_INFO.save(deps.storage, farm_info)?;
    STAKERS_INFO.save(deps.storage, info.sender, &staker_info)?;

    res = res
        .add_message(transfer)
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
    if amount.is_zero() {
        return Err(ContractError::Std(StdError::generic_err(
            "InvalidZeroAmount: Withdraw amount is 0",
        )));
    }

    let farm_info = &mut FARM_INFO.load(deps.storage)?;
    let current_phase_index: usize = farm_info.current_phase_index as usize;
    let mut staker_info =
        if let Some(staker_info) = STAKERS_INFO.may_load(deps.storage, info.sender.clone())? {
            staker_info
        } else {
            return Err(ContractError::Std(StdError::generic_err(
                "Unauthorized: Sender is not staker",
            )));
        };

    if staker_info.amount < amount {
        return Err(ContractError::Std(StdError::generic_err(
            "InsufficientFunds: Withdraw amount exceeds staked amount",
        )));
    }

    let mut res = Response::new();
    let current_time = env.block.time.seconds();

    let reward_amount = claim_all_reward(farm_info, &mut staker_info, current_time);

    // If reward amount is greater than 0, transfer reward token to the sender
    if reward_amount > Uint128::zero() {
        match farm_info.reward_token.clone() {
            TokenInfo::Token { contract_addr } => {
                let transfer_reward = wasm_execute(
                    contract_addr.to_string(),
                    &Cw20ExecuteMsg::Transfer {
                        recipient: info.sender.to_string(),
                        amount: reward_amount,
                    },
                    vec![],
                )?;
                res = res.add_message(transfer_reward);
            }
            TokenInfo::NativeToken { denom } => {
                res = res.add_message(CosmosMsg::Bank(BankMsg::Send {
                    to_address: info.sender.to_string(),
                    amount: coins(reward_amount.into(), denom),
                }))
            }
        };
    }

    // Withdraw staked token from the farm contract by using cw20 transfer message
    let withdraw = wasm_execute(
        farm_info.staked_token.to_string(),
        &Cw20ExecuteMsg::Transfer {
            recipient: info.sender.to_string(),
            amount,
        },
        vec![],
    )?;
    // Decrease staked token balance
    farm_info.staked_token_balance -= amount;

    // Update staker amount
    staker_info.amount -= amount;
    staker_info.reward_debt[current_phase_index] =
        staker_info.amount * farm_info.phases_info[current_phase_index].accrued_token_per_share;
    staker_info.joined_phase = current_phase_index as u64;

    // Check if staker amount is zero, remove staker info from storage
    if staker_info.amount == Uint128::zero() {
        STAKERS_INFO.remove(deps.storage, info.sender);
    } else {
        // Update staker info
        STAKERS_INFO.save(deps.storage, info.sender, &staker_info)?;
    }
    // Save farm info
    FARM_INFO.save(deps.storage, farm_info)?;

    res = res
        .add_message(withdraw)
        .add_attribute("method", "withdraw")
        .add_attribute("withdraw_amount", amount.to_string())
        .add_attribute("harvest_reward_amount", reward_amount.to_string())
        .add_attribute("current_time", current_time.to_string());

    Ok(res)
}

// Harvest reward token from the farm contract to the sender
pub fn execute_harvest(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let mut staker_info =
        if let Some(staker_info) = STAKERS_INFO.may_load(deps.storage, info.sender.clone())? {
            staker_info
        } else {
            return Err(ContractError::Std(StdError::generic_err(
                "Unauthorized: Only staker can harvest reward",
            )));
        };
    let farm_info = &mut FARM_INFO.load(deps.storage)?;

    let current_time = env.block.time.seconds();
    let current_phase_index: usize = farm_info.current_phase_index as usize;

    let reward_amount = claim_all_reward(farm_info, &mut staker_info, current_time);

    // Check if there is any reward to harvest
    if reward_amount == Uint128::zero() {
        return Err(ContractError::Std(StdError::generic_err(
            "InsufficientFunds: Reward amount is zero",
        )));
    }

    staker_info.reward_debt[current_phase_index] =
        staker_info.amount * farm_info.phases_info[current_phase_index].accrued_token_per_share;
    staker_info.joined_phase = current_phase_index as u64;

    STAKERS_INFO.save(deps.storage, info.sender.clone(), &staker_info)?;
    FARM_INFO.save(deps.storage, farm_info)?;
    // Init response
    let mut res = Response::new();

    // Transfer reward token to the sender
    match &farm_info.reward_token {
        TokenInfo::Token { contract_addr } => {
            let transfer_reward = wasm_execute(
                contract_addr.to_string(),
                &Cw20ExecuteMsg::Transfer {
                    recipient: info.sender.to_string(),
                    amount: reward_amount,
                },
                vec![],
            )?;
            res = res.add_message(transfer_reward);
        }
        TokenInfo::NativeToken { denom } => {
            res = res.add_message(CosmosMsg::Bank(BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: coins(reward_amount.into(), denom),
            }))
        }
    };

    let res = res
        .add_attribute("method", "harvest")
        .add_attribute("reward_amount", reward_amount.to_string())
        .add_attribute("current_time", current_time.to_string());

    Ok(res)
}

pub fn execute_add_phase(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    new_start_time: u64,
    new_end_time: u64,
    whitelist: Addr,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;
    // Check if the message sender is the owner of the contract
    if config.farm_owner != info.sender {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: Only owner can add new phase",
        )));
    }

    // Validate time range
    validate_time_range(env, new_start_time, new_end_time)?;

    let mut farm_info: FarmInfo = FARM_INFO.load(deps.storage)?;
    let phases_length = farm_info.phases_info.len();
    let current_phase_index: usize = farm_info.current_phase_index as usize;

    // Not allow add new phase when new start time is less than end time of the current phase
    if new_start_time < farm_info.phases_info[current_phase_index].end_time {
        return Err(ContractError::Std(StdError::generic_err(
            "New start time is less than end time of the current phase",
        )));
    }

    // Not allow add new phase when previous phase is not active yet
    if phases_length - 1 > current_phase_index {
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
    let current_phase_index = farm_info.current_phase_index;

    // Not allow active phase when current phase index is equal to farm info length
    // If sender want to active new phase, they have to add new phase first
    if farm_info.phases_info.len() == current_phase_index as usize {
        return Err(ContractError::Std(StdError::generic_err(
            "Invalid action: Add new phase first",
        )));
    }

    // Get current time
    let current_time = env.block.time.seconds();

    // Not allow activating phase when current time is less than end time of the current phase
    // or greater than start time of the phase to be activated
    if current_time < farm_info.phases_info[current_phase_index as usize].end_time
        || current_time > farm_info.phases_info[current_phase_index as usize + 1].start_time
    {
        return Err(ContractError::Std(StdError::generic_err(
            "Current time is not in range of the phase to be activated",
        )));
    }

    // Not allow activating phase when reward balance of this phase is zero
    if farm_info.phases_info[current_phase_index as usize + 1]
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
    let phase_info = &mut farm_info.phases_info[current_phase_index as usize];

    // Update reward ratio and time
    phase_info.update_reward_ratio_and_time(phase_info.end_time, staked_token_balance);

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

// validate time when creating new farm
pub fn validate_time_range(
    env: Env,
    start_time: u64,
    end_time: u64,
) -> Result<(), ContractError> {
    // Not allow start time is greater than end time
    if start_time >= end_time {
        return Err(ContractError::Std(StdError::generic_err(
            "Start time is greater than end time",
        )));
    }

    // Not allow to create a farm when current time is greater than start time
    if env.block.time.seconds() > start_time {
        return Err(ContractError::Std(StdError::generic_err(
            "Current time is greater than start time",
        )));
    }

    Ok(())
}
