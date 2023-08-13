use cosmwasm_std::{
    to_binary, Addr, BalanceResponse, BankQuery, Deps, Env, QuerierWrapper, QueryRequest,
    StdResult, Uint128, WasmQuery,
};
use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg};

use crate::{
    execute::claim_all_reward,
    state::{
        FarmInfo, PendingRewardResponse, StakerInfo, StakerInfoResponse, FARM_INFO, STAKERS_INFO,
    },
};
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

pub fn query_farm_info(deps: Deps) -> StdResult<FarmInfo> {
    FARM_INFO.load(deps.storage)
}

pub fn query_pending_reward(
    deps: Deps,
    env: Env,
    address: String,
) -> StdResult<PendingRewardResponse> {
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

pub fn query_total_lp_token_staked(deps: Deps) -> StdResult<Uint128> {
    Ok(FARM_INFO.load(deps.storage)?.staked_token_balance)
}

pub fn query_staker_info(deps: Deps, address: String) -> StdResult<StakerInfoResponse> {
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