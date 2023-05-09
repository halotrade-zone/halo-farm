#![cfg(test)]
mod tests {
    use crate::contract::{instantiate, query};
    use crate::msg::{InstantiateMsg, QueryMsg};
    use crate::state::ConfigResponse;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{
        attr, coin, from_binary, to_binary, Addr, Api, CosmosMsg, OwnedDeps, Reply, ReplyOn,
        Response, StdError, SubMsg, SubMsgResponse, SubMsgResult, Uint128, WasmMsg,
    };
    use cw20_base::{msg::ExecuteMsg as Cw20ExecuteMsg, msg::QueryMsg as Cw20QueryMsg};
    const MOCK_1000_000_000_HALO_LP_TOKEN_AMOUNT: u128 = 1_000_000_000_000_000;
    const MOCK_TRANSACTION_FEE: u128 = 5000;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};



    // create a lp token contract
    // create pool contract by factory contract
    // deposit some lp token to the pool contract
    // withdraw some lp token from the pool contract
    mod execute_deposit_and_withdraw {
        use std::time::{SystemTime, UNIX_EPOCH};

        use cosmwasm_std::{testing::mock_dependencies, Uint128, Addr, Coin, BlockInfo};
        use cw20::{Cw20ExecuteMsg, BalanceResponse};
        use cw_multi_test::Executor;
        use halo_pool::state::RewardTokenInfo;

        use crate::tests::{env_setup::env::{instantiate_contracts, ADMIN, NATIVE_DENOM_2}, integration_test::tests::{MOCK_1000_000_000_HALO_LP_TOKEN_AMOUNT, MOCK_TRANSACTION_FEE}};

        #[test]
        fn proper_operation() {
            // get integration test app and contracts
            let (mut app, contracts) = instantiate_contracts();

            // get pool factory contract
            let factory_contract = &contracts[0].contract_addr;
            // get halo lp token contract
            let lp_token_contract = &contracts[1].contract_addr;

            // Mint 1000 tokens to USER_1
            let mint_msg: Cw20ExecuteMsg = Cw20ExecuteMsg::Mint {
                recipient: ADMIN.to_string(),
                amount: Uint128::from(MOCK_1000_000_000_HALO_LP_TOKEN_AMOUNT),
            };

            // Execute minting
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(lp_token_contract.clone()),
                &mint_msg,
                &[Coin {
                    amount: Uint128::from(MOCK_TRANSACTION_FEE),
                    denom: NATIVE_DENOM_2.to_string(),
                }],
            );

            assert!(response.is_ok());

            // query balance of USER_1 in cw20 base token contract
            let balance: BalanceResponse = app
                .wrap()
                .query_wasm_smart(
                    lp_token_contract.clone(),
                    &cw20::Cw20QueryMsg::Balance {
                        address: ADMIN.to_string(),
                    },
                )
                .unwrap();
            // It should be 1000_000_000 lp token as minting happened
            assert_eq!(
                balance.balance,
                Uint128::from(MOCK_1000_000_000_HALO_LP_TOKEN_AMOUNT)
            );

            // native token info
            let native_token_info = RewardTokenInfo::NativeToken {
                denom: NATIVE_DENOM_2.to_string(),
            };

            // get current block time
            let current_block_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

            // create pool contract by factory contract
            let create_pool_msg = crate::msg::ExecuteMsg::CreatePool {
                staked_token: lp_token_contract.clone(),
                reward_token: native_token_info,
                reward_per_second: Uint128::from(1u128),
                start_time: current_block_time,
                end_time: current_block_time + 10 + 1000,
            };

            // Execute create pool
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(factory_contract.clone()),
                &create_pool_msg,
                &[Coin {
                    amount: Uint128::from(MOCK_TRANSACTION_FEE),
                    denom: NATIVE_DENOM_2.to_string(),
                }],
            );
            println!("response: {:?}", response);
            assert!(response.is_ok());

        }
    }
}
