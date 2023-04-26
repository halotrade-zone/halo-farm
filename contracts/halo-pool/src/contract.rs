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

use crate::{msg::InstantiateMsg, state::{PoolInfo, POOL_INFO, RewardTokenInfo}};

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