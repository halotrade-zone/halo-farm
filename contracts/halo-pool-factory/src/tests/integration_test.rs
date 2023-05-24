#![cfg(test)]
mod tests {
    const MOCK_1000_000_000_HALO_LP_TOKEN_AMOUNT: u128 = 1_000_000_000_000_000;
    // Mock information for native token
    const MOCK_1000_000_000_NATIVE_TOKEN_AMOUNT: u128 = 2_000_000_000_000_000_000_000_000_000;
    const MOCK_TRANSACTION_FEE: u128 = 5000;

    // create a lp token contract
    // create pool contract by factory contract
    // deposit some lp token to the pool contract
    // withdraw some lp token from the pool contract
    mod execute_deposit_and_withdraw {
        use std::time::{SystemTime, UNIX_EPOCH};

        use cosmwasm_std::{
            from_binary, to_binary, Addr, BalanceResponse as BankBalanceResponse, BankQuery, Coin,
            Querier, QueryRequest, Uint128,
        };
        use cw20::{BalanceResponse, Cw20ExecuteMsg};
        use cw_multi_test::Executor;
        use halo_pool::state::{PoolInfo, RewardTokenAsset, RewardTokenInfo};

        use crate::{
            msg::QueryMsg,
            state::FactoryPoolInfo,
            tests::{
                env_setup::env::{instantiate_contracts, ADMIN, NATIVE_DENOM_2},
                integration_test::tests::{
                    MOCK_1000_000_000_HALO_LP_TOKEN_AMOUNT, MOCK_1000_000_000_NATIVE_TOKEN_AMOUNT,
                    MOCK_TRANSACTION_FEE,
                },
            },
        };
        use halo_pool::msg::{ExecuteMsg as PoolExecuteMsg, QueryMsg as PoolQueryMsg};

        #[test]
        fn proper_operation() {
            // get integration test app and contracts
            let (mut app, contracts) = instantiate_contracts();

            // Mint 1000 native tokens NATIVE_DENOM_2 to ADMIN
            app.sudo(cw_multi_test::SudoMsg::Bank(
                cw_multi_test::BankSudo::Mint {
                    to_address: ADMIN.to_string(),
                    amount: vec![Coin {
                        amount: Uint128::from(MOCK_1000_000_000_NATIVE_TOKEN_AMOUNT),
                        denom: NATIVE_DENOM_2.to_string(),
                    }],
                },
            ))
            .unwrap();

            // query balance of USER_1 in native token
            let req: QueryRequest<BankQuery> = QueryRequest::Bank(BankQuery::Balance {
                address: ADMIN.to_string(),
                denom: NATIVE_DENOM_2.to_string(),
            });
            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let balance: BankBalanceResponse = from_binary(&res).unwrap();

            // It should be 1_000_000_000 NATIVE_DENOM_2 as minting happened
            assert_eq!(
                balance.amount.amount,
                Uint128::from(2000000000000000500000000000u128)
            );

            // get pool factory contract
            let factory_contract = &contracts[0].contract_addr;
            // get halo lp token contract
            let lp_token_contract = &contracts[1].contract_addr;

            // Mint 1000 tokens to ADMIN
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

            // query balance of ADMIN in cw20 base token contract
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
                reward_token: native_token_info.clone(),
                start_time: current_block_time,
                end_time: current_block_time + 10,
                whitelist: vec![Addr::unchecked(ADMIN.to_string())],
            };

            // Execute create pool
            let response_create_pool = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(factory_contract.clone()),
                &create_pool_msg,
                &[],
            );
            assert!(response_create_pool.is_ok());

            // query pool contract address
            let pool_info: FactoryPoolInfo = app
                .wrap()
                .query_wasm_smart(
                    factory_contract.clone(),
                    &crate::msg::QueryMsg::Pool { pool_id: 1u64 },
                )
                .unwrap();

            // assert pool info
            assert_eq!(
                pool_info,
                FactoryPoolInfo {
                    staked_token: lp_token_contract.to_string(),
                    reward_token: native_token_info.clone(),
                    start_time: current_block_time,
                    end_time: current_block_time + 10,
                }
            );

            let reward_asset_info = RewardTokenInfo::NativeToken {
                denom: NATIVE_DENOM_2.to_string(),
            };

            // add reward balance to pool contract
            let add_reward_balance_msg = PoolExecuteMsg::AddRewardBalance {
                asset: RewardTokenAsset {
                    info: reward_asset_info,
                    amount: Uint128::from(1000u128),
                },
            };

            // Execute add reward balance
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract2"),
                &add_reward_balance_msg,
                &[Coin {
                    amount: Uint128::from(1000u128),
                    denom: NATIVE_DENOM_2.to_string(),
                }],
            );
            assert!(response.is_ok());

            // query pool info after adding reward balance
            let pool_info: PoolInfo = app
                .wrap()
                .query_wasm_smart("contract2", &PoolQueryMsg::Pool {})
                .unwrap();

            // assert pool info
            assert_eq!(
                pool_info,
                PoolInfo {
                    staked_token: lp_token_contract.to_string(),
                    reward_token: native_token_info.clone(),
                    reward_per_second: Uint128::from(1000u128),
                    start_time: current_block_time,
                    end_time: current_block_time + 10,
                    whitelist: vec![Addr::unchecked(ADMIN.to_string())],
                }
            );

            // query all pools
            let pools: Vec<FactoryPoolInfo> = app
                .wrap()
                .query_wasm_smart(
                    Addr::unchecked(factory_contract.clone()),
                    &QueryMsg::Pools {
                        start_after: None,
                        limit: None,
                    },
                )
                .unwrap();

            // assert pool info
            assert_eq!(
                pools,
                vec![FactoryPoolInfo {
                    staked_token: lp_token_contract.to_string(),
                    reward_token: native_token_info,
                    start_time: current_block_time,
                    end_time: current_block_time + 10,
                }]
            );

            // Approve cw20 token to pool contract
            let approve_msg: Cw20ExecuteMsg = Cw20ExecuteMsg::IncreaseAllowance {
                spender: "contract2".to_string(), // Pool Contract
                amount: Uint128::from(1000u128),
                expires: None,
            };

            // Execute approve
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(lp_token_contract.clone()),
                &approve_msg,
                &[Coin {
                    amount: Uint128::from(MOCK_TRANSACTION_FEE),
                    denom: NATIVE_DENOM_2.to_string(),
                }],
            );

            assert!(response.is_ok());

            // deposit some lp token to the pool contract
            let deposit_msg = PoolExecuteMsg::Deposit {
                amount: Uint128::from(1000u128),
            };

            // Execute deposit
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract2"),
                &deposit_msg,
                &[],
            );

            println!("responseeeee: {:?}", response);

            assert!(response.is_ok());
        }
    }
}
