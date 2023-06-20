#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, to_binary, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, SubMsg, Uint128, WasmMsg, QuerierWrapper, Addr, QueryRequest, WasmQuery, BalanceResponse, BankQuery, Decimal, StdError
};

use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg, BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg};


// version info for migration info
const CONTRACT_NAME: &str = "crates.io:halo-pool";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

use crate::{
    error::ContractError,
    formulas::{get_multiplier, calc_reward_amount, update_pool},
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::{
        PoolInfo, RewardTokenAsset, TokenInfo, LAST_REWARD_TIME, POOL_INFO, POOL_INFOS, STAKERS_INFO, ACCRUED_TOKEN_PER_SHARE, PHASES_REWARD_BALANCE, StakerRewardAssetInfo, CONFIG, Config, PoolInfos, PhasesRewardBalance,
    },
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        halo_factory_owner: info.sender,
    };

    // Init pool index
    let current_pool_index = 0u64;
    // Init pool info
    let pool_info = &PoolInfo {
        staked_token: deps.api.addr_validate(&msg.staked_token)?.to_string(),
        reward_token: msg.reward_token.clone(),
        reward_per_second: Decimal::zero(), // will be updated when admin adding reward balance
        start_time: msg.start_time,
        end_time: msg.end_time,
        pool_limit_per_user: msg.pool_limit_per_user,
        whitelist: msg.whitelist,
    };

    // Init pool infos
    POOL_INFOS.save(deps.storage, &PoolInfos {
        current_pool_index,
        pool_infos: vec![pool_info.clone()],
    })?;

    // Save config
    CONFIG.save(deps.storage, &config)?;

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
        ("pool_limit_per_user", &msg.pool_limit_per_user.unwrap_or(Uint128::zero()).to_string()),
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
        ExecuteMsg::UpdatePoolLimitPerUser { new_pool_limit_per_user } => execute_update_pool_limit_per_user(deps, info, new_pool_limit_per_user),
        ExecuteMsg::ExtendEndTime { new_end_time } => execute_extend_end_time(deps, info, new_end_time),
    }
}

pub fn execute_add_reward_balance(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    phase_index: u64,
    asset: RewardTokenAsset,
) -> Result<Response, ContractError> {
    let current_time = env.block.time;
    let last_reward_time = LAST_REWARD_TIME.load(deps.storage)?;
    // Get pool infos
    let mut pool_infos = POOL_INFOS.load(deps.storage)?;
    // Get current pool index
    let current_pool_index = POOL_INFOS.load(deps.storage)?.current_pool_index;
    // Get current pool info in pool infos
    let pool_info = POOL_INFOS.load(deps.storage)?.pool_infos[current_pool_index as usize].clone();
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

    if let TokenInfo::Token { contract_addr } = pool_info.reward_token.clone() {
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

    // Get the reward token balance of the pool in multiple phases.
    let mut phases_reward_balance = PHASES_REWARD_BALANCE.load(deps.storage).unwrap_or(
        PhasesRewardBalance {
            current_phase_index: 0u64,
            reward_balance: vec![RewardTokenAsset {
                info: pool_info.reward_token.clone(),
                amount: Uint128::zero(),
            }],
        }
    );

    // Get reward token balance of the pool in current phase
    let mut reward_balance = phases_reward_balance.reward_balance[phase_index as usize].clone();

    // Update reward balance
    reward_balance.amount += asset.amount;

    // Save reward balance to phases reward balance
    phases_reward_balance.reward_balance[phase_index as usize] = reward_balance.clone();

    // Update reward_per_second base on new reward balance
    let new_reward_per_second = Decimal::from_ratio(reward_balance.amount, pool_info.end_time - pool_info.start_time).floor();

    let new_pool_info = PoolInfo {
        staked_token: pool_info.staked_token.clone(),
        reward_token: pool_info.reward_token,
        reward_per_second: new_reward_per_second,
        start_time: pool_info.start_time,
        end_time: pool_info.end_time,
        pool_limit_per_user: pool_info.pool_limit_per_user,
        whitelist: pool_info.whitelist,
    };

    // Get staked token balance from pool contract
    let staked_token_supply = query_token_balance(&deps.querier, pool_info.staked_token, env.contract.address.clone())?;

    // Save phases reward balance
    PHASES_REWARD_BALANCE.save(deps.storage, &phases_reward_balance)?;

    // Save pool info to pool infos in current pool index
    pool_infos.pool_infos[current_pool_index as usize] = new_pool_info.clone();
    POOL_INFOS.save(deps.storage, &pool_infos)?;

    // Update last reward time to start time
    LAST_REWARD_TIME.save(deps.storage, &pool_info.start_time)?;

    // update pool
    let (new_accrued_token_per_share, new_last_reward_time)
    = update_pool(
        pool_info.end_time,
        pool_info.reward_per_second,
        staked_token_supply,
        accrued_token_per_share,
        current_time.seconds(),
        last_reward_time
    );

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
    // Get pool infos
    let mut pool_infos = POOL_INFOS.load(deps.storage)?;
    // Get current pool index
    let current_pool_index = POOL_INFOS.load(deps.storage)?.current_pool_index;
    // Get current pool info in pool infos
    let pool_info = POOL_INFOS.load(deps.storage)?.pool_infos[current_pool_index as usize].clone();
    // Init reward amount
    let mut reward_amount = Uint128::zero();
    // Init new accrued token per share
    let mut new_accrued_token_per_share = Decimal::zero();
    // Init new last reward time
    let mut new_last_reward_time = current_time.seconds();
    // get staker info
    let mut staker_info = STAKERS_INFO
        .may_load(deps.storage, info.sender.clone())?
        .unwrap_or(StakerRewardAssetInfo {
            amount: Uint128::zero(),
            reward_debt: Uint128::zero(),
            joined_phases: current_pool_index,
        });
    // Check pool limit per user
    if let Some(pool_limit_per_user) = pool_info.pool_limit_per_user {
        if staker_info.amount + amount > pool_limit_per_user {
            return Err(ContractError::Std(StdError::generic_err(
                "Unauthorized: Deposit amount exceeds pool limit per user",
            )));
        }
    }

    let mut res = Response::new();

    // get staker joined phases
    let staker_joined_phases = staker_info.joined_phases;

    // If staker has joined previous phases, loop down all pool infos to get reward per second from current pool index to staker joined phases
    if staker_joined_phases < current_pool_index {
        for i in staker_joined_phases..current_pool_index {
            // Get last reward time
            let last_reward_time = LAST_REWARD_TIME.load(deps.storage)?;
            // Get accrued token per share
            let accrued_token_per_share = ACCRUED_TOKEN_PER_SHARE.load(deps.storage)?;
            // Get pool info from pool infos
            let pool_info = POOL_INFOS.load(deps.storage)?.pool_infos[i as usize].clone();
            // Get staked token balance from pool contract
            let staked_token_supply = query_token_balance(&deps.querier, pool_info.staked_token.clone(), env.contract.address.clone())?;

            // update pool
            (new_accrued_token_per_share, new_last_reward_time)
            = update_pool(
                pool_info.end_time,
                pool_info.reward_per_second,
                staked_token_supply,
                accrued_token_per_share,
                current_time.seconds(),
                last_reward_time
            );

            // Save accrued token per share
            ACCRUED_TOKEN_PER_SHARE.save(deps.storage, &new_accrued_token_per_share)?;

            // Save last reward time
            LAST_REWARD_TIME.save(deps.storage, &new_last_reward_time)?;

            reward_amount += calc_reward_amount(
                staker_info.amount,
                new_accrued_token_per_share,
                staker_info.reward_debt,
            );
            // Update staker info
            staker_info.reward_debt = staker_info.amount * new_accrued_token_per_share;
        }
    } else {
        // Get last reward time
        let last_reward_time = LAST_REWARD_TIME.load(deps.storage)?;
        // Get accrued token per share
        let accrued_token_per_share = ACCRUED_TOKEN_PER_SHARE.load(deps.storage)?;

        // Get staked token balance from pool contract
        let staked_token_supply = query_token_balance(&deps.querier, pool_info.staked_token.clone(), env.contract.address.clone())?;

        // update pool
        (new_accrued_token_per_share, new_last_reward_time)
        = update_pool(
            pool_info.end_time,
            pool_info.reward_per_second,
            staked_token_supply,
            accrued_token_per_share,
            current_time.seconds(),
            last_reward_time
        );

        // Save accrued token per share
        ACCRUED_TOKEN_PER_SHARE.save(deps.storage, &new_accrued_token_per_share)?;
        // Save last reward time
        LAST_REWARD_TIME.save(deps.storage, &new_last_reward_time)?;

        reward_amount = calc_reward_amount(
            staker_info.amount,
            new_accrued_token_per_share,
            staker_info.reward_debt,
        );
    }

    // If there is any reward token in the pool, transfer reward token to the sender
    if reward_amount > Uint128::zero() {
        let transfer_reward = match pool_info.reward_token {
            TokenInfo::Token { contract_addr } => {
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr,
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: info.sender.to_string(),
                        amount: reward_amount,
                    })?,
                    funds: vec![],
                }))
            }
            TokenInfo::NativeToken { denom } => SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: vec![coin(reward_amount.into(), denom)],
            })),
        };
        res = res.add_submessage(transfer_reward);
    }

    // // Decrease phase reward balance
    // let mut phases_reward_balance = PHASES_REWARD_BALANCE.load(deps.storage)?;
    // let mut reward_balance = phases_reward_balance.reward_balance[current_pool_index as usize].clone();
    // reward_balance.amount -= reward_amount;

    // // Save reward balance to phases reward balance
    // phases_reward_balance.reward_balance[current_pool_index as usize] = reward_balance.clone();

    // // Save phases reward balance
    // PHASES_REWARD_BALANCE.save(deps.storage, &phases_reward_balance)?;

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

    // Update staker info
    staker_info.amount += amount;
    staker_info.reward_debt = staker_info.amount * new_accrued_token_per_share;
    staker_info.joined_phases = current_pool_index;

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
    // Get pool infos
    let mut pool_infos = POOL_INFOS.load(deps.storage)?;
    // Get current pool index
    let current_pool_index = POOL_INFOS.load(deps.storage)?.current_pool_index;
    // Get Staker info
    let mut staker_info = STAKERS_INFO
        .may_load(deps.storage, info.sender.clone())?
        .unwrap_or(StakerRewardAssetInfo {
            amount: Uint128::zero(),
            reward_debt: Uint128::zero(),
            joined_phases: current_pool_index,
        });

    if staker_info.amount == Uint128::zero() {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: Only staker can withdraw",
        )));
    }
    // Init reward amount
    let mut reward_amount = Uint128::zero();
    // Get current time
    let current_time = env.block.time;
    // Get current pool info in pool infos
    let pool_info = POOL_INFOS.load(deps.storage)?.pool_infos[current_pool_index as usize].clone();
    // Init new accrued token per share
    let mut new_accrued_token_per_share = Decimal::zero();
    // Init new last reward time
    let mut new_last_reward_time = current_time.seconds();

    let current_staker_amount = staker_info.amount;

    // Check staker amount is greater than withdraw amount
    if current_staker_amount < amount {
        return Err(ContractError::Std(StdError::generic_err(
            "InsufficientFunds: Withdraw amount exceeds staked amount",
        )));
    }

    let mut res = Response::new();

    // get staker joined phases
    let staker_joined_phases = staker_info.joined_phases;

    // If staker has joined previous phases, loop down all pool infos to get reward per second from current pool index to staker joined phases
    if staker_joined_phases < current_pool_index {
        for i in staker_joined_phases..current_pool_index {
            // Get last reward time
            let last_reward_time = LAST_REWARD_TIME.load(deps.storage)?;
            // Get accrued token per share
            let accrued_token_per_share = ACCRUED_TOKEN_PER_SHARE.load(deps.storage)?;
            // Get pool info from pool infos
            let pool_info = POOL_INFOS.load(deps.storage)?.pool_infos[i as usize].clone();
            // Get staked token balance from pool contract
            let staked_token_supply = query_token_balance(&deps.querier, pool_info.staked_token.clone(), env.contract.address.clone())?;

            // update pool
            (new_accrued_token_per_share, new_last_reward_time)
            = update_pool(
                pool_info.end_time,
                pool_info.reward_per_second,
                staked_token_supply,
                accrued_token_per_share,
                current_time.seconds(),
                last_reward_time
            );

            // Save accrued token per share
            ACCRUED_TOKEN_PER_SHARE.save(deps.storage, &new_accrued_token_per_share)?;

            // Save last reward time
            LAST_REWARD_TIME.save(deps.storage, &new_last_reward_time)?;

            reward_amount += calc_reward_amount(
                staker_info.amount,
                new_accrued_token_per_share,
                staker_info.reward_debt,
            );
            // Update staker info
            staker_info.reward_debt = staker_info.amount * new_accrued_token_per_share;
        }
    } else {
        // Get last reward time
        let last_reward_time = LAST_REWARD_TIME.load(deps.storage)?;
        // Get accrued token per share
        let accrued_token_per_share = ACCRUED_TOKEN_PER_SHARE.load(deps.storage)?;
        // Get staked token balance from pool contract
        let staked_token_supply = query_token_balance(&deps.querier, pool_info.staked_token.clone(), env.contract.address.clone())?;

        // update pool
        (new_accrued_token_per_share, new_last_reward_time)
            = update_pool(
                pool_info.end_time,
                pool_info.reward_per_second,
                staked_token_supply,
                accrued_token_per_share,
                current_time.seconds(),
                last_reward_time
            );
        // Save accrued token per share
        ACCRUED_TOKEN_PER_SHARE.save(deps.storage, &new_accrued_token_per_share)?;
        // Save last reward time
        LAST_REWARD_TIME.save(deps.storage, &new_last_reward_time)?;

        reward_amount = calc_reward_amount(
            staker_info.amount,
            new_accrued_token_per_share,
            staker_info.reward_debt,
        );
    }

    // Transfer reward token to the sender
    let transfer_reward = match pool_info.reward_token {
        TokenInfo::Token { contract_addr } => {
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: info.sender.to_string(),
                    amount: reward_amount,
                })?,
                funds: vec![],
            }))
        }
        TokenInfo::NativeToken { denom } => SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![coin(reward_amount.into(), denom)],
        })),
    };

    // // Decrease phase reward balance
    // let mut phases_reward_balance = PHASES_REWARD_BALANCE.load(deps.storage)?;
    // let mut reward_balance = phases_reward_balance.reward_balance[current_pool_index as usize].clone();
    // reward_balance.amount -= reward_amount;

    // // Save reward balance to phases reward balance
    // phases_reward_balance.reward_balance[current_pool_index as usize] = reward_balance.clone();

    // // Save phases reward balance
    // PHASES_REWARD_BALANCE.save(deps.storage, &phases_reward_balance)?;

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
    staker_info.reward_debt = staker_info.amount * new_accrued_token_per_share;

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
        .add_attribute("method", "harvest and withdraw");

    Ok(res)
}

// Harvest reward token from the pool to the sender
pub fn execute_harvest(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // Get current pool index
    let current_pool_index = POOL_INFOS.load(deps.storage)?.current_pool_index;
    // Get Staker info
    let mut staker_info = STAKERS_INFO
        .may_load(deps.storage, info.sender.clone())?
        .unwrap_or(StakerRewardAssetInfo {
            amount: Uint128::zero(),
            reward_debt: Uint128::zero(),
            joined_phases: current_pool_index,
        });

    if staker_info.amount == Uint128::zero() {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: Only staker can harvest reward",
        )));
    }
    // Get current time
    let current_time = env.block.time;

    // Get current pool info in pool infos
    let pool_info = POOL_INFOS.load(deps.storage)?.pool_infos[current_pool_index as usize].clone();
    // Init reward amount
    let mut reward_amount = Uint128::zero();
    // get staker joined phases
    let staker_joined_phases = staker_info.joined_phases;
    // Init new accrued token per share
    let mut new_accrued_token_per_share = Decimal::zero();
    // Init new last reward time
    let mut new_last_reward_time;

    // If staker has joined previous phases, loop down all pool infos to get reward per second from current pool index to staker joined phases
    if staker_joined_phases < current_pool_index {
        for i in staker_joined_phases..current_pool_index {
            // Get last reward time
            let last_reward_time = LAST_REWARD_TIME.load(deps.storage)?;
            // Get accrued token per share
            let accrued_token_per_share = ACCRUED_TOKEN_PER_SHARE.load(deps.storage)?;
            // Get pool info from pool infos
            let pool_info = POOL_INFOS.load(deps.storage)?.pool_infos[i as usize].clone();
            // Get staked token balance from pool contract
            let staked_token_supply = query_token_balance(&deps.querier, pool_info.staked_token.clone(), env.contract.address.clone())?;

            // update pool
            (new_accrued_token_per_share, new_last_reward_time)
            = update_pool(
                pool_info.end_time,
                pool_info.reward_per_second,
                staked_token_supply,
                accrued_token_per_share,
                current_time.seconds(),
                last_reward_time
            );

            // Save accrued token per share
            ACCRUED_TOKEN_PER_SHARE.save(deps.storage, &new_accrued_token_per_share)?;

            // Save last reward time
            LAST_REWARD_TIME.save(deps.storage, &new_last_reward_time)?;

            reward_amount += calc_reward_amount(
                staker_info.amount,
                new_accrued_token_per_share,
                staker_info.reward_debt,
            );
        }
    } else {
        // Get last reward time
        let last_reward_time = LAST_REWARD_TIME.load(deps.storage)?;
        // Get accrued token per share
        let accrued_token_per_share = ACCRUED_TOKEN_PER_SHARE.load(deps.storage)?;

        // Get staked token balance from pool contract
        let staked_token_supply = query_token_balance(&deps.querier, pool_info.staked_token.clone(), env.contract.address)?;

        // update pool
        (new_accrued_token_per_share, new_last_reward_time)
        = update_pool(
            pool_info.end_time,
            pool_info.reward_per_second,
            staked_token_supply,
            accrued_token_per_share,
            current_time.seconds(),
            last_reward_time
        );
        // Save accrued token per share
        ACCRUED_TOKEN_PER_SHARE.save(deps.storage, &new_accrued_token_per_share)?;
        // Save last reward time
        LAST_REWARD_TIME.save(deps.storage, &new_last_reward_time)?;

        reward_amount = calc_reward_amount(
            staker_info.amount,
            new_accrued_token_per_share,
            staker_info.reward_debt,
        );
    }

    // Check if there is any reward to harvest
    if reward_amount == Uint128::zero() {
        return Err(ContractError::Std(StdError::generic_err(
            "InsufficientFunds: Reward amount is zero",
        )));
    }

    // Transfer reward token to the sender
    let transfer = match pool_info.reward_token {
        TokenInfo::Token { contract_addr } => {
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: info.sender.to_string(),
                    amount: reward_amount,
                })?,
                funds: vec![],
            }))
        }
        TokenInfo::NativeToken { denom } => SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![coin(reward_amount.into(), denom)],
        })),
    };

    // // Decrease phase reward balance
    // let mut phases_reward_balance = PHASES_REWARD_BALANCE.load(deps.storage)?;
    // let mut reward_balance = phases_reward_balance.reward_balance[current_pool_index as usize].clone();
    // reward_balance.amount -= reward_amount;

    // // Save reward balance to phases reward balance
    // phases_reward_balance.reward_balance[current_pool_index as usize] = reward_balance.clone();

    // // Save phases reward balance
    // PHASES_REWARD_BALANCE.save(deps.storage, &phases_reward_balance)?;

    // Update staker reward debt
    staker_info.reward_debt = staker_info.amount * new_accrued_token_per_share;
    // Update staker info
    STAKERS_INFO.save(deps.storage, info.sender, &staker_info)?;

    let res = Response::new()
        .add_submessage(transfer)
        .add_attribute("method", "harvest");

    Ok(res)
}

fn execute_update_pool_limit_per_user(
    deps: DepsMut,
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
    // Get pool info
    let mut pool_info: PoolInfo = POOL_INFO.load(deps.storage)?;
    // Update pool limit per user
    pool_info.pool_limit_per_user = Some(new_pool_limit_per_user);
    // Save pool info
    POOL_INFO.save(deps.storage, &pool_info)?;

    let res = Response::new()
        .add_attribute("method", "update_pool_limit_per_user")
        .add_attribute("new_pool_limit_per_user", new_pool_limit_per_user.to_string());

    Ok(res)
}

pub fn execute_extend_end_time(
    deps: DepsMut,
    info: MessageInfo,
    new_end_time: u64,
) -> Result<Response, ContractError> {
    // Get config
    let config: Config = CONFIG.load(deps.storage)?;
    // Check if the message sender is the owner of the contract
    if config.halo_factory_owner != info.sender {
        return Err(ContractError::Std(StdError::generic_err(
            "Unauthorized: Only owner can extend end time",
        )));
    }
    // Get pool infos
    let mut pool_infos: PoolInfos = POOL_INFOS.load(deps.storage)?;

    // Increase current pool index
    pool_infos.current_pool_index += 1;

    // Update end time of the current pool
    pool_infos.pool_infos[pool_infos.current_pool_index as usize].end_time = new_end_time;

    // Save pool infos
    POOL_INFOS.save(deps.storage, &pool_infos)?;

    let res = Response::new()
        .add_attribute("method", "extend_end_time")
        .add_attribute("new_end_time", new_end_time.to_string());

    Ok(res)
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
        QueryMsg::TotalStaked {  } => Ok(to_binary(&query_total_lp_token_staked(deps, env)?)?),
    }
}

fn query_pool_info(deps: Deps) -> Result<PoolInfo, ContractError> {
    // Get current pool index
    let current_pool_index = POOL_INFOS.load(deps.storage)?.current_pool_index;
    // Get current pool info in pool infos
    let pool_info = POOL_INFOS.load(deps.storage)?.pool_infos[current_pool_index as usize].clone();
    let res = PoolInfo {
        ..pool_info
    };
    Ok(res)
}

fn query_pending_reward(deps: Deps, env: Env, address: String) -> Result<RewardTokenAsset, ContractError> {
    // Get current time
    let current_time = env.block.time;
    // Get last reward time
    let last_reward_time = LAST_REWARD_TIME.load(deps.storage)?;
    // Get accrued token per share
    let mut accrued_token_per_share = ACCRUED_TOKEN_PER_SHARE.load(deps.storage)?;
    // Get current pool index
    let current_pool_index = POOL_INFOS.load(deps.storage)?.current_pool_index;
    // Get current pool info in pool infos
    let pool_info = POOL_INFOS.load(deps.storage)?.pool_infos[current_pool_index as usize].clone();
    // Get staker info
    let staker_info = STAKERS_INFO
        .may_load(deps.storage, Addr::unchecked(address))?
        .unwrap_or(StakerRewardAssetInfo {
            amount: Uint128::zero(),
            reward_debt: Uint128::zero(),
            joined_phases: 0u64,
        });

    // Get staked token balance from pool contract
    let staked_token_supply = query_token_balance(&deps.querier, pool_info.staked_token.clone(), env.contract.address)?;
    // get staker joined phases
    let staker_joined_phases = staker_info.joined_phases;

    // Check if there is any staked token in the pool
    if staked_token_supply == Uint128::zero() {
        // No staked token in the pool, save last reward time and return
        let res = RewardTokenAsset {
            info: pool_info.reward_token,
            amount: Uint128::zero(),
        };
        return Ok(res);
    }

    if staker_joined_phases < current_pool_index {
        // Init reward amount
        let mut reward_amount = Uint128::zero();
        for i in staker_joined_phases..current_pool_index {
            // Get last reward time
            let last_reward_time = LAST_REWARD_TIME.load(deps.storage)?;
            // Get accrued token per share
            let mut accrued_token_per_share = ACCRUED_TOKEN_PER_SHARE.load(deps.storage)?;
            // Get pool info from pool infos
            let pool_info = POOL_INFOS.load(deps.storage)?.pool_infos[i as usize].clone();

            let multiplier = get_multiplier(
                last_reward_time,
                current_time.seconds(),
                pool_info.end_time,
            );

            let reward = Decimal::new(multiplier.into()) * pool_info.reward_per_second;
            accrued_token_per_share = accrued_token_per_share + (reward / Decimal::new(staked_token_supply.into()));


            reward_amount += calc_reward_amount(
                staker_info.amount,
                accrued_token_per_share,
                staker_info.reward_debt,
            );
        }
        let res = RewardTokenAsset {
            info: pool_info.reward_token,
            amount: reward_amount,
        };

        Ok(res)
    } else {
        let multiplier = get_multiplier(
            last_reward_time,
            current_time.seconds(),
            pool_info.end_time,
        );

        let reward = Decimal::new(multiplier.into()) * pool_info.reward_per_second;
        accrued_token_per_share = accrued_token_per_share + (reward / Decimal::new(staked_token_supply.into()));


        let reward_amount = calc_reward_amount(
            staker_info.amount,
            accrued_token_per_share,
            staker_info.reward_debt,
        );

        let res = RewardTokenAsset {
            info: pool_info.reward_token,
            amount: reward_amount,
        };

        Ok(res)
    }
}

fn query_total_lp_token_staked(deps: Deps, env: Env) -> Result<Uint128, ContractError> {
    // Get pool info
    let pool_info: PoolInfo = POOL_INFO.load(deps.storage)?;
    let staked_token_supply = query_token_balance(&deps.querier, pool_info.staked_token, env.contract.address.clone())?;
    Ok(staked_token_supply)
}
