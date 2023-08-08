use crate::error::ContractError;
use crate::state::{Config, ConfigResponse, FactoryFarmInfo, CONFIG};
use crate::{
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::{FARMS, NUMBER_OF_FARMS},
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, QuerierWrapper,
    QueryRequest, Reply, ReplyOn, Response, StdError, StdResult, SubMsg, Uint128, WasmMsg,
    WasmQuery,
};
use cw2::set_contract_version;
use cw_utils::parse_reply_instantiate_data;
use halo_farm::msg::InstantiateMsg as FarmInstantiateMsg;
use halo_farm::msg::QueryMsg as FarmQueryMsg;
use halo_farm::state::{PhasesInfo, TokenInfo};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:halo-farm-factory";
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
        farm_code_id: msg.farm_code_id,
    };

    // init NUMBER_OF_FARMS to 0
    NUMBER_OF_FARMS.save(deps.storage, &0u64)?;

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
            farm_code_id,
        } => execute_update_config(deps, env, info, owner, farm_code_id),
        ExecuteMsg::CreateFarm {
            staked_token,
            reward_token,
            start_time,
            end_time,
            phases_limit_per_user,
            whitelist,
        } => execute_create_farm(
            deps,
            env,
            info,
            staked_token,
            reward_token,
            start_time,
            end_time,
            phases_limit_per_user,
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
    farm_code_id: Option<u64>,
) -> Result<Response, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    // permission check
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    // update owner if provided
    if let Some(owner) = owner.clone() {
        config.owner = deps.api.addr_validate(&owner)?;
    }

    // update farm_code_id if provided
    if let Some(farm_code_id) = farm_code_id {
        config.farm_code_id = farm_code_id;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "update_config")
        .add_attribute("owner", owner.unwrap())
        .add_attribute("farm_code_id", farm_code_id.unwrap().to_string()))
}

// Only owner can execute it
#[allow(clippy::too_many_arguments)]
pub fn execute_create_farm(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    staked_token: Addr,
    reward_token: TokenInfo,
    start_time: u64,
    end_time: u64,
    phases_limit_per_user: Option<Uint128>,
    whitelist: Addr,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;
    // get current time
    let current_time = env.block.time.seconds();
    // permission check
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    // Not allow start time is greater than end time
    if start_time >= end_time {
        return Err(ContractError::Std(StdError::generic_err(
            "Start time is greater than end time",
        )));
    }

    // Not allow to create a farm when current time is greater than start time
    if current_time > start_time {
        return Err(ContractError::Std(StdError::generic_err(
            "Current time is greater than start time",
        )));
    }

    Ok(Response::new()
        .add_attributes(vec![
            ("method", "create_farm"),
            ("halo_factory_owner", info.sender.as_str()),
            ("staked_token", staked_token.as_str()),
            ("reward_token", &format!("{}", reward_token)),
            ("start_time", start_time.to_string().as_str()),
            ("end_time", end_time.to_string().as_str()),
            (
                "phases_limit_per_user",
                &format!("{:?}", phases_limit_per_user),
            ),
            ("whitelist", &format!("{:?}", whitelist)),
        ])
        .add_submessage(SubMsg {
            id: 1,
            gas_limit: None,
            msg: CosmosMsg::Wasm(WasmMsg::Instantiate {
                code_id: config.farm_code_id,
                funds: vec![],
                admin: Some(config.owner.to_string()),
                label: "farm".to_string(),
                msg: to_binary(&FarmInstantiateMsg {
                    staked_token,
                    reward_token,
                    start_time,
                    end_time,
                    phases_limit_per_user,
                    farm_owner: info.sender,
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

    let farm_contract = &reply.contract_address;
    let phases_info = query_phases_info(&deps.querier, Addr::unchecked(farm_contract))?;

    let farm_key = NUMBER_OF_FARMS.load(deps.storage)? + 1;

    FARMS.save(
        deps.storage,
        farm_key,
        &FactoryFarmInfo {
            staked_token: phases_info.staked_token.clone(),
            reward_token: phases_info.reward_token,
            start_time: phases_info.phases_info[phases_info.current_phase_index as usize]
                .start_time,
            end_time: phases_info.phases_info[phases_info.current_phase_index as usize].end_time,
            phases_limit_per_user: phases_info.phases_limit_per_user,
        },
    )?;

    // increase farm count
    NUMBER_OF_FARMS.save(deps.storage, &(farm_key))?;

    Ok(Response::new().add_attributes(vec![
        ("action", "reply_on_create_farm_success"),
        ("farm_id", farm_key.to_string().as_str()),
        ("farm_contract_addr", farm_contract),
        ("staked_token_addr", phases_info.staked_token.as_ref()),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Farm { farm_id } => to_binary(&query_farm_info(deps, farm_id)?),
        QueryMsg::Farms { start_after, limit } => {
            to_binary(&query_farms(deps, start_after, limit)?)
        }
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state: Config = CONFIG.load(deps.storage)?;
    let resp = ConfigResponse {
        owner: state.owner.to_string(),
        farm_code_id: state.farm_code_id,
    };

    Ok(resp)
}

pub fn query_farm_info(deps: Deps, farm_id: u64) -> StdResult<FactoryFarmInfo> {
    FARMS.load(deps.storage, farm_id)
}

pub fn query_farms(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<FactoryFarmInfo>> {
    let start_after = start_after.unwrap_or(0);
    let limit = limit.unwrap_or(30) as usize;
    let farm_count = NUMBER_OF_FARMS.load(deps.storage)?;
    let farms = (start_after..farm_count)
        .map(|farm_id| FARMS.load(deps.storage, farm_id + 1))
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?;

    Ok(farms)
}

fn query_phases_info(querier: &QuerierWrapper, farm_contract: Addr) -> StdResult<PhasesInfo> {
    let phases_info: PhasesInfo = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: farm_contract.to_string(),
        msg: to_binary(&FarmQueryMsg::Phases {})?,
    }))?;
    Ok(phases_info)
}
