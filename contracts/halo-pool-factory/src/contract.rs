use crate::error::ContractError;
use crate::state::{Config, ConfigResponse, FactoryPoolInfo, CONFIG};
use crate::{
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::{NUMBER_OF_POOLS, POOLS},
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, QuerierWrapper,
    QueryRequest, Reply, ReplyOn, Response, StdResult, SubMsg, Uint128, WasmMsg, WasmQuery,
};
use cw2::set_contract_version;
use cw_utils::parse_reply_instantiate_data;
use halo_pool::msg::InstantiateMsg as PoolInstantiateMsg;
use halo_pool::msg::QueryMsg as PoolQueryMsg;
use halo_pool::state::{PoolInfo, TokenInfo};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:halo-pool-factory";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        owner: info.sender,
        pool_code_id: msg.pool_code_id,
    };

    // init NUMBER_OF_POOLS to 0
    NUMBER_OF_POOLS.save(deps.storage, &0u64)?;

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig {
            owner,
            pool_code_id,
        } => execute_update_config(deps, env, info, owner, pool_code_id),
        ExecuteMsg::CreatePool {
            staked_token,
            reward_token,
            start_time,
            end_time,
            pool_limit_per_user,
            whitelist,
        } => execute_create_pool(
            deps,
            env,
            info,
            staked_token,
            reward_token,
            start_time,
            end_time,
            pool_limit_per_user,
            whitelist,
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
) -> Result<Response, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    // permission check
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    // update owner if provided
    if let Some(owner) = owner {
        config.owner = deps.api.addr_validate(&owner)?;
    }

    // update pool_code_id if provided
    if let Some(pool_code_id) = pool_code_id {
        config.pool_code_id = pool_code_id;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

// Only owner can execute it
#[allow(clippy::too_many_arguments)]
pub fn execute_create_pool(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    staked_token: String,
    reward_token: TokenInfo,
    start_time: u64,
    end_time: u64,
    pool_limit_per_user: Option<Uint128>,
    whitelist: Vec<Addr>,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;

    // permission check
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    // validate address format
    let _ = deps.api.addr_validate(&staked_token)?;

    Ok(Response::new()
        .add_attributes(vec![
            ("action", "create_pool"),
            ("halo_factory_owner", info.sender.as_str()),
            ("staked_token", staked_token.as_str()),
            ("reward_token", &format!("{}", reward_token)),
            ("start_time", start_time.to_string().as_str()),
            ("end_time", end_time.to_string().as_str()),
            ("whitelist", &format!("{:?}", whitelist)),
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
                    start_time,
                    end_time,
                    pool_limit_per_user,
                    pool_owner: info.sender,
                    whitelist,
                })?,
            }),
            reply_on: ReplyOn::Success,
        }))
}

/// This just stores the result for future query
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response> {
    let reply = parse_reply_instantiate_data(msg).unwrap();

    let pool_contract = &reply.contract_address;
    let pool_info = query_pair_info_from_pair(&deps.querier, Addr::unchecked(pool_contract))?;

    let pool_key = NUMBER_OF_POOLS.load(deps.storage)? + 1;

    POOLS.save(
        deps.storage,
        pool_key,
        &FactoryPoolInfo {
            staked_token: pool_info.staked_token.clone(),
            reward_token: pool_info.reward_token,
            start_time: pool_info.start_time,
            end_time: pool_info.end_time,
            pool_limit_per_user: pool_info.pool_limit_per_user,
        },
    )?;

    // increase pool count
    NUMBER_OF_POOLS.save(deps.storage, &(pool_key))?;

    Ok(Response::new().add_attributes(vec![
        ("action", "reply_on_create_pool_success"),
        ("pool_id", pool_key.to_string().as_str()),
        ("pool_contract_addr", pool_contract),
        ("staked_token_addr", &pool_info.staked_token),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Pool { pool_id } => to_binary(&query_pool_info(deps, pool_id)?),
        QueryMsg::Pools { start_after, limit } => {
            to_binary(&query_pools(deps, start_after, limit)?)
        }
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state: Config = CONFIG.load(deps.storage)?;
    let resp = ConfigResponse {
        owner: state.owner.to_string(),
        pool_code_id: state.pool_code_id,
    };

    Ok(resp)
}

pub fn query_pool_info(deps: Deps, pool_id: u64) -> StdResult<FactoryPoolInfo> {
    POOLS.load(deps.storage, pool_id)
}

pub fn query_pools(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<FactoryPoolInfo>> {
    let start_after = start_after.unwrap_or(0);
    let limit = limit.unwrap_or(30) as usize;
    let pool_count = NUMBER_OF_POOLS.load(deps.storage)?;

    let pools = (start_after..pool_count)
        .map(|pool_id| POOLS.load(deps.storage, pool_id + 1))
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?;

    Ok(pools)
}

fn query_pair_info_from_pair(querier: &QuerierWrapper, pair_contract: Addr) -> StdResult<PoolInfo> {
    let pair_info: PoolInfo = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: pair_contract.to_string(),
        msg: to_binary(&PoolQueryMsg::Pool {})?,
    }))?;

    Ok(pair_info)
}
