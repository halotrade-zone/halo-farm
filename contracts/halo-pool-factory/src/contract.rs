#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, ReplyOn, Response,
    StdError, StdResult, SubMsg, WasmMsg, Uint128,
};
use cw2::set_contract_version;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use halo_pool::state::RewardTokenInfo;
use halo_pool::msg::InstantiateMsg as PoolInstantiateMsg;
use crate::state::{Config, CONFIG, ConfigResponse,};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:halo-pool-factory";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        owner: deps.api.addr_canonicalize(info.sender.as_str())?,
        pool_code_id: msg.pool_code_id,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateConfig {
            owner,
            pool_code_id,
        } => execute_update_config(deps, env, info, owner, pool_code_id),
        ExecuteMsg::CreatePool {
            staked_token,
            reward_token,
            reward_per_second,
            start_time,
            end_time,
        } => execute_create_pool(
            deps,
            env,
            info,
            staked_token,
            reward_token,
            reward_per_second,
            start_time,
            end_time,
        ),
    }
}

// Only owner can execute it
pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    owner: Option<String>,
    pool_code_id: Option<u64>,
) -> StdResult<Response> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    // permission check
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner {
        return Err(StdError::generic_err("unauthorized"));
    }

    if let Some(owner) = owner {
        // validate address format
        let _ = deps.api.addr_validate(&owner)?;

        config.owner = deps.api.addr_canonicalize(&owner)?;
    }

    if let Some(pool_code_id) = pool_code_id {
        config.pool_code_id = pool_code_id;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

// Anyone can execute it to create a new pool
pub fn execute_create_pool(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    staked_token: String ,
    reward_token: RewardTokenInfo,
    reward_per_second: Uint128,
    start_time: u64,
    end_time: u64,
) -> StdResult<Response> {
    let config: Config = CONFIG.load(deps.storage)?;

    // permission check
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner {
        return Err(StdError::generic_err("unauthorized"));
    }

    // validate address format
    let _ = deps.api.addr_validate(&staked_token)?;

    Ok(Response::new()
        .add_attributes(vec![
            ("action", "create_pool"),
            ("staked_token", staked_token.as_str()),
            ("reward_token", &format!("{}", reward_token)),
            ("reward_per_second", reward_per_second.to_string().as_str()),
            ("start_time", start_time.to_string().as_str()),
            ("end_time", end_time.to_string().as_str()),
        ])
        .add_submessage(SubMsg {
            id: 1,
            gas_limit: None,
            msg: CosmosMsg::Wasm(WasmMsg::Instantiate {
                code_id: config.pool_code_id,
                funds: vec![],
                admin: Some(env.contract.address.to_string()),
                label: "pair".to_string(),
                msg: to_binary(&PoolInstantiateMsg {
                    staked_token,
                    reward_token,
                    reward_per_second,
                    start_time,
                    end_time,
                })?,
            }),
            reply_on: ReplyOn::Success,
        }))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state: Config = CONFIG.load(deps.storage)?;
    let resp = ConfigResponse {
        owner: deps.api.addr_humanize(&state.owner)?.to_string(),
        pool_code_id: state.pool_code_id,
    };

    Ok(resp)
}