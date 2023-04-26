use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use crate::contract::{instantiate, query};
use crate::msg::{InstantiateMsg, QueryMsg};
use crate::state::ConfigResponse;
use cosmwasm_std::{
    attr, coin, from_binary, to_binary, Addr, Api, CosmosMsg, OwnedDeps, Reply, ReplyOn, Response,
    StdError, SubMsg, SubMsgResponse, SubMsgResult, Uint128, WasmMsg,
};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();

    let msg = InstantiateMsg {
        pool_code_id: 321u64,
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!("addr0000".to_string(), config_res.owner);
    assert_eq!(321u64, config_res.pool_code_id);
}