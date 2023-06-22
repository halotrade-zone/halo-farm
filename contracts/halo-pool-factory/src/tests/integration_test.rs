#![cfg(test)]
mod tests {
    const _MOCK_1000_000_000_HALO_LP_TOKEN_AMOUNT: u128 = 1_000_000_000_000_000;
    const MOCK_1000_HALO_LP_TOKEN_AMOUNT: u128 = 1_000_000_000;
    const MOCK_1000_HALO_REWARD_TOKEN_AMOUNT: u128 = 1_000_000_000_000_000_000_000;
    const _MOCK_150_000_000_HALO_LP_TOKEN_AMOUNT: u128 = 150_000_000_000_000;
    const MOCK_150_HALO_LP_TOKEN_AMOUNT: u128 = 150_000_000;
    const INIT_1000_000_NATIVE_BALANCE_2: u128 = 1_000_000_000_000u128;
    const ADD_1000_NATIVE_BALANCE_2: u128 = 1_000_000_000u128;

    // create a lp token contract
    // create pool contract by factory contract
    // deposit some lp token to the pool contract
    // withdraw some lp token from the pool contract
    mod execute_deposit_and_withdraw {
        use std::str::FromStr;

        use cosmwasm_std::{
            from_binary, to_binary, Addr, BalanceResponse as BankBalanceResponse, BankQuery,
            BlockInfo, Coin, Decimal, Querier, QueryRequest, Uint128, WasmQuery,
        };
        use cw20::{BalanceResponse, Cw20ExecuteMsg};
        use cw_multi_test::Executor;
        use halo_pool::state::{PoolInfo, RewardTokenAsset, TokenInfo, StakerRewardAssetInfo};

        use crate::{
            msg::QueryMsg,
            state::FactoryPoolInfo,
            tests::{
                env_setup::env::{instantiate_contracts, ADMIN, NATIVE_DENOM_2, USER_1},
                integration_test::tests::{
                    ADD_1000_NATIVE_BALANCE_2, INIT_1000_000_NATIVE_BALANCE_2,
                    MOCK_1000_HALO_LP_TOKEN_AMOUNT, MOCK_1000_HALO_REWARD_TOKEN_AMOUNT,
                    MOCK_150_HALO_LP_TOKEN_AMOUNT,
                },
            },
        };
        use halo_pool::msg::{ExecuteMsg as PoolExecuteMsg, QueryMsg as PoolQueryMsg};
/*
        #[test]
        fn proper_operation() {
            // get integration test app and contracts
            let (mut app, contracts) = instantiate_contracts();

            // query balance of ADMIN in native token
            let req: QueryRequest<BankQuery> = QueryRequest::Bank(BankQuery::Balance {
                address: ADMIN.to_string(),
                denom: NATIVE_DENOM_2.to_string(),
            });
            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let balance: BankBalanceResponse = from_binary(&res).unwrap();

            // It should be 1_000_000 NATIVE_DENOM_2 as minting happened
            assert_eq!(
                balance.amount.amount,
                Uint128::from(INIT_1000_000_NATIVE_BALANCE_2)
            );

            // get pool factory contract
            let factory_contract = &contracts[0].contract_addr;
            // get halo lp token contract
            let lp_token_contract = &contracts[1].contract_addr;

            // Mint 1000 HALO LP tokens to ADMIN
            let mint_msg: Cw20ExecuteMsg = Cw20ExecuteMsg::Mint {
                recipient: ADMIN.to_string(),
                amount: Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT),
            };

            // Execute minting
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(lp_token_contract.clone()),
                &mint_msg,
                &[],
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
            // It should be 1000 lp token as minting happened
            assert_eq!(
                balance.balance,
                Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT)
            );

            // native token info
            let native_token_info = TokenInfo::NativeToken {
                denom: NATIVE_DENOM_2.to_string(),
            };

            // get current block time
            let current_block_time = app.block_info().time.seconds();

            // create pool contract by factory contract
            let create_pool_msg = crate::msg::ExecuteMsg::CreatePool {
                staked_token: lp_token_contract.clone(),
                reward_token: native_token_info.clone(),
                start_time: current_block_time,
                end_time: current_block_time + 100,
                pool_limit_per_user: None,
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
                    end_time: current_block_time + 100,
                    pool_limit_per_user: None,
                }
            );

            let reward_asset_info = TokenInfo::NativeToken {
                denom: NATIVE_DENOM_2.to_string(),
            };

            // add reward balance to pool contract
            let add_reward_balance_msg = PoolExecuteMsg::AddRewardBalance {
                phase_index: 0u64,
                asset: RewardTokenAsset {
                    info: reward_asset_info,
                    amount: Uint128::from(ADD_1000_NATIVE_BALANCE_2),
                },
            };

            // Execute add reward balance
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &add_reward_balance_msg,
                &[Coin {
                    amount: Uint128::from(ADD_1000_NATIVE_BALANCE_2),
                    denom: NATIVE_DENOM_2.to_string(),
                }],
            );

            assert!(response.is_ok());

            // query pool info after adding reward balance
            let pool_info: PoolInfo = app
                .wrap()
                .query_wasm_smart("contract3", &PoolQueryMsg::Pool {})
                .unwrap();

            // assert pool info
            assert_eq!(
                pool_info,
                PoolInfo {
                    staked_token: lp_token_contract.to_string(),
                    reward_token: native_token_info.clone(),
                    reward_per_second: Decimal::from_str("10000000").unwrap(), // 10_000_000 (10 NATIVE_DENOM_2)
                    start_time: current_block_time,
                    end_time: current_block_time + 100,
                    pool_limit_per_user: None,
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
                    end_time: current_block_time + 100,
                    pool_limit_per_user: None,
                }]
            );

            // Approve cw20 token to pool contract
            let approve_msg: Cw20ExecuteMsg = Cw20ExecuteMsg::IncreaseAllowance {
                spender: "contract3".to_string(), // Pool Contract
                amount: Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT),
                expires: None,
            };

            // Execute approve
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(lp_token_contract.clone()),
                &approve_msg,
                &[],
            );

            assert!(response.is_ok());

            // deposit lp token to the pool contract
            let deposit_msg = PoolExecuteMsg::Deposit {
                amount: Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT),
            };

            // Execute deposit
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &deposit_msg,
                &[],
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

            // It should be 0 lp token as deposit happened
            assert_eq!(balance.balance, Uint128::zero());

            // query balance of pool contract in cw20 base token contract
            let balance: BalanceResponse = app
                .wrap()
                .query_wasm_smart(
                    lp_token_contract.clone(),
                    &cw20::Cw20QueryMsg::Balance {
                        address: "contract3".to_string(),
                    },
                )
                .unwrap();

            // It should be MOCK_1000_HALO_LP_TOKEN_AMOUNT lp token as deposit happened
            assert_eq!(
                balance.balance,
                Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT)
            );

            // change block time increase 6 seconds to make phase active
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(6),
                height: app.block_info().height + 1,
                chain_id: app.block_info().chain_id,
            });

            // Query pending reward
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward: RewardTokenAsset = from_binary(&res).unwrap();

            // It should be 60_000_000 as reward is accrued
            assert_eq!(
                pending_reward,
                RewardTokenAsset {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(60000000u128)
                }
            );

            // Harvest reward
            let harvest_msg = PoolExecuteMsg::Harvest {};

            // Execute harvest
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &harvest_msg,
                &[],
            );

            assert!(response.is_ok());

            // query balance of ADMIN in native token
            let req: QueryRequest<BankQuery> = QueryRequest::Bank(BankQuery::Balance {
                address: ADMIN.to_string(),
                denom: NATIVE_DENOM_2.to_string(),
            });
            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let balance: BankBalanceResponse = from_binary(&res).unwrap();

            assert_eq!(
                balance.amount.amount,
                Uint128::from(
                    INIT_1000_000_NATIVE_BALANCE_2 - ADD_1000_NATIVE_BALANCE_2
                        + pending_reward.amount.u128()
                )
            );

            // withdraw some lp token from the pool contract
            let withdraw_msg = PoolExecuteMsg::Withdraw {
                amount: Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT),
            };

            // change block time increase 2 seconds to make phase active
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(2),
                height: app.block_info().height + 1,
                chain_id: app.block_info().chain_id,
            });

            // Query pending reward
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward: RewardTokenAsset = from_binary(&res).unwrap();

            // It should be 20_000_000 as reward is accrued
            assert_eq!(
                pending_reward,
                RewardTokenAsset {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(20_000_000u128)
                }
            );

            // Execute withdraw
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &withdraw_msg,
                &[],
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

            // It should be 1000 lp token as deposit happened
            assert_eq!(
                balance.balance,
                Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT)
            );

            // query balance of ADMIN in native token
            let req: QueryRequest<BankQuery> = QueryRequest::Bank(BankQuery::Balance {
                address: ADMIN.to_string(),
                denom: NATIVE_DENOM_2.to_string(),
            });
            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let balance: BankBalanceResponse = from_binary(&res).unwrap();

            assert_eq!(
                balance.amount.amount,
                Uint128::from(
                    INIT_1000_000_NATIVE_BALANCE_2 - ADD_1000_NATIVE_BALANCE_2
                        + 60000000u128
                        + pending_reward.amount.u128()
                )
            );
        }
*/

        // Create pool contract by factory contract
        // ----- Phase 0 -----
        // Add 1000 NATIVE_2 reward balance amount to pool contract by ADMIN in phase 0
        // with end time 100 seconds -> 10 NATIVE_2 per second
        // Deposit 1000 lp token to the pool contract by ADMIN
        // Deposit 500 lp token to the pool contract by USER_1
        // Harvest reward by ADMIN after 2 seconds -> (1000 / (1000 + 500)) * 2 * 10 = 13.333 NATIVE_2
        // Harvest reward by USER_1 after 2 seconds -> (500 / (1000 + 500)) * 2 * 10 = 6.666 NATIVE_2
        // - Withdraw 50% lp token from the pool contract by ADMIN after 6 seconds
        //   -> Lp token balance in ADMIN wallet: 500 LP token
        //   -> Reward balance: 4s: (1000 / (1000 + 500)) * (6 - 2) * 10  = 26,66 NATIVE_2
        // - Withdraw 100% lp token from the pool contract by USER_1 after 8 seconds
        //   -> Lp token balance in USER_1 wallet: 500 LP token
        //   -> Reward balance: 4s: (500 / (1000 + 500)) * (6 - 2) * 10  = 13,33 NATIVE_2
        //                      2s: (500 / (1000 - 500 + 500)) * (8 - 6) * 10  = 10 NATIVE_2
        //                      = 23,33 NATIVE_2
        // Harvest reward by ADMIN after 10 seconds
        //   -> Reward balance: 2s: (500 / 1000) * 2 * 10  = 10 NATIVE_2
        //                      2s: (500 / (1000 - 500)) * 2 * 10  = 20 NATIVE_2
        // Harvest reward by USER_1 after 12 seconds (can not be done as all lp token is withdrawn)
        //
        // ADMIN deposit 500 lp token to the pool contract after 14 seconds
        //   -> ADMIN lp token balance: 1000 LP token
        //   -> Reward balance: 4s: (500 / 500) * 4 * 10  = 40 NATIVE_2
        // USER_1 deposit 150 lp token to the pool contract after 16 seconds
        //   -> USER_1 lp token balance: 150 LP token
        // Harvest reward by ADMIN after 18 seconds
        //   -> Reward balance: 2s: (500 / 500) * 2 * 10  = 20 NATIVE_2
        //                      2s: (1000 / (1000 + 150)) * 2 * 10  = 17,39 NATIVE_2
        //                      = 37,39 NATIVE_2
        // Harvest reward by USER_1 after 18 seconds
        //   -> Reward balance: 2s: (150 / (1000 + 150)) * 2 * 10  = 2,608 NATIVE_2
        //
        // Harvest reward at the end time by ADMIN (100s - 18s = 82s)
        //
        // At this time: ADMIN lp token balance: 1000 LP token
        //               USER_1 lp token balance: 150 LP token
        //   -> Reward balance: ADMIN: 999_147_391_304 NATIVE_2
        //                      USER_1: 32_608_695 NATIVE_2
        //
        // Extend end time by ADMIN more 80 seconds in phase 0 to create phase 1
        // with start time = previous end time + 10s
        // with end time = previous end time + 90s
        //
        //   -> Reward balance: 82s: (1000 / (1000 + 150)) * 82 * 10  = 713,043 NATIVE_2
        // Query reward at the end time by USER_1 (100s - 18s = 82s)
        //   -> Reward balance: 82s: (150 / (1000 + 150)) * 82 * 10  = 106,956 NATIVE_2
        // Ended phase 0: ADMIN lp token balance: 1000 LP token
        //              USER_1 lp token balance: 150 LP token
        //              Reward balance: ADMIN: 999_860_434_782 NATIVE_2 (All reward already harvested)
        //                              USER_1: 106_956_522 NATIVE_2 (Not claim yet)
        //                                      32_608_695 NATIVE_2 (Already claim)
        //
        // ----- Phase 1 -----
        // Increase simulation time more 5 seconds
        // Add 1000 NATIVE_2 reward balance amount to pool contract by ADMIN in phase 1
        // -> NATIVE_2 ADMIN Balance: 998_860_434_782 NATIVE_2
        // with end time 80 seconds -> 12.5 NATIVE_2 per second
        // Harvest reward by ADMIN after 25 seconds
        //   -> Reward balance: 25s: (1000 / (1000 + 150)) * (25-5) * 12.5  = 217,391 NATIVE_2
        // Harvest reward by USER_1 after 25 seconds
        //   -> Reward balance: 25s: (150 / (1000 + 150)) * (25-5) * 12.5  = 32,608 NATIVE_2
        //                      100s in Phase 0: (Not claim yet) = 106,956 NATIVE_2
        //                      = 139,564 NATIVE_2
        #[test]
        fn proper_operation_with_multiple_users() {
            // get integration test app and contracts
            let (mut app, contracts) = instantiate_contracts();
            // ADMIN already has 1_000_000 NATIVE_DENOM_2 as initial balance in instantiate_contracts()
            // get pool factory contract
            let factory_contract = &contracts[0].contract_addr;
            // get halo lp token contract
            let lp_token_contract = &contracts[1].contract_addr;

            // get current block time
            let current_block_time = app.block_info().time.seconds();

            // Mint 1000 HALO LP tokens to ADMIN
            let mint_msg: Cw20ExecuteMsg = Cw20ExecuteMsg::Mint {
                recipient: ADMIN.to_string(),
                amount: Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT),
            };

            // Execute minting
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(lp_token_contract.clone()),
                &mint_msg,
                &[],
            );

            assert!(response.is_ok());

            // Mint 500 HALO LP tokens to USER_1
            let mint_msg: Cw20ExecuteMsg = Cw20ExecuteMsg::Mint {
                recipient: USER_1.to_string(),
                amount: Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT / 2),
            };

            // Execute minting
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(lp_token_contract.clone()),
                &mint_msg,
                &[],
            );

            assert!(response.is_ok());

            // native token info
            let native_token_info = TokenInfo::NativeToken {
                denom: NATIVE_DENOM_2.to_string(),
            };

            // create pool contract by factory contract
            let create_pool_msg = crate::msg::ExecuteMsg::CreatePool {
                staked_token: lp_token_contract.clone(),
                reward_token: native_token_info.clone(),
                start_time: current_block_time,
                end_time: current_block_time + 100,
                pool_limit_per_user: None,
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

            let reward_asset_info = TokenInfo::NativeToken {
                denom: NATIVE_DENOM_2.to_string(),
            };

            // add reward balance to pool contract
            let add_reward_balance_msg = PoolExecuteMsg::AddRewardBalance {
                phase_index: 0u64,
                asset: RewardTokenAsset {
                    info: reward_asset_info,
                    amount: Uint128::from(ADD_1000_NATIVE_BALANCE_2),
                },
            };

            // Execute add reward balance
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &add_reward_balance_msg,
                &[Coin {
                    amount: Uint128::from(ADD_1000_NATIVE_BALANCE_2),
                    denom: NATIVE_DENOM_2.to_string(),
                }],
            );

            assert!(response.is_ok());

            // query pool info after adding reward balance
            let pool_info: PoolInfo = app
                .wrap()
                .query_wasm_smart("contract3", &PoolQueryMsg::Pool {})
                .unwrap();

            // assert pool info
            assert_eq!(
                pool_info,
                PoolInfo {
                    staked_token: lp_token_contract.to_string(),
                    reward_token: native_token_info.clone(),
                    reward_per_second: Decimal::from_str("10000000").unwrap(), // 10_000_000 (10 NATIVE_DENOM_2)
                    start_time: current_block_time,
                    end_time: current_block_time + 100,
                    pool_limit_per_user: None,
                    whitelist: vec![Addr::unchecked(ADMIN.to_string())],
                }
            );

            // Approve cw20 token to pool contract msg
            let approve_msg: Cw20ExecuteMsg = Cw20ExecuteMsg::IncreaseAllowance {
                spender: "contract3".to_string(), // Pool Contract
                amount: Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT),
                expires: None,
            };

            // Execute approve by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(lp_token_contract.clone()),
                &approve_msg,
                &[],
            );

            assert!(response.is_ok());

            // Deposit lp token to the pool contract to execute deposit msg
            let deposit_msg = PoolExecuteMsg::Deposit {
                amount: Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT),
            };

            // Execute deposit by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &deposit_msg,
                &[],
            );
            assert!(response.is_ok());

            // Execute approve by USER_1
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked(lp_token_contract.clone()),
                &approve_msg,
                &[],
            );

            assert!(response.is_ok());

            // Deposit lp token to the pool contract to execute deposit msg
            let deposit_msg = PoolExecuteMsg::Deposit {
                amount: Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT / 2),
            };

            // Execute deposit by USER_1
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &deposit_msg,
                &[],
            );

            assert!(response.is_ok());

            // change block time increase 2 seconds to make 2 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(2),
                height: app.block_info().height + 2,
                chain_id: app.block_info().chain_id,
            });

            // Query pending reward by ADMIN
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_admin_2s: RewardTokenAsset = from_binary(&res).unwrap();

            // It should be 13333333 as reward is accrued
            assert_eq!(
                pending_reward_admin_2s,
                RewardTokenAsset {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(13_333_333u128)
                }
            );

            // Query pending reward by USER_1
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: USER_1.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_user1_2s: RewardTokenAsset = from_binary(&res).unwrap();

            // It should be 6666666 as reward is accrued
            assert_eq!(
                pending_reward_user1_2s,
                RewardTokenAsset {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(6_666_666u128)
                }
            );

            // Harvest reward by ADMIN
            let harvest_msg = PoolExecuteMsg::Harvest {};

            // Execute harvest by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &harvest_msg,
                &[],
            );

            assert!(response.is_ok());

            // query balance of ADMIN in native token
            let req: QueryRequest<BankQuery> = QueryRequest::Bank(BankQuery::Balance {
                address: ADMIN.to_string(),
                denom: NATIVE_DENOM_2.to_string(),
            });
            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let balance: BankBalanceResponse = from_binary(&res).unwrap();

            assert_eq!(
                balance.amount.amount,
                Uint128::from(
                    INIT_1000_000_NATIVE_BALANCE_2 - ADD_1000_NATIVE_BALANCE_2
                        + pending_reward_admin_2s.amount.u128()
                )
            );

            // Harvest reward by USER_1
            let harvest_msg = PoolExecuteMsg::Harvest {};

            // Execute harvest by USER_1
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &harvest_msg,
                &[],
            );

            assert!(response.is_ok());

            // query balance of USER_1 in native token
            let req: QueryRequest<BankQuery> = QueryRequest::Bank(BankQuery::Balance {
                address: USER_1.to_string(),
                denom: NATIVE_DENOM_2.to_string(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let balance: BankBalanceResponse = from_binary(&res).unwrap();

            assert_eq!(
                balance.amount.amount,
                Uint128::from(pending_reward_user1_2s.amount.u128())
            );

            // change block time increase 4 seconds to make 6 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(4),
                height: app.block_info().height + 4,
                chain_id: app.block_info().chain_id,
            });

            // Query pending reward by ADMIN
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_admin_6s: RewardTokenAsset = from_binary(&res).unwrap();

            // It should be 26666666 as reward is accrued
            assert_eq!(
                pending_reward_admin_6s,
                RewardTokenAsset {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(26_666_666u128)
                }
            );

            // Withdraw 50% lp token from the pool contract by ADMIN
            let withdraw_msg = PoolExecuteMsg::Withdraw {
                amount: Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT / 2),
            };

            // Execute withdraw by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &withdraw_msg,
                &[],
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

            // It should be 500 lp token
            assert_eq!(
                balance.balance,
                Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT / 2)
            );

            // query balance of ADMIN in native token
            let req: QueryRequest<BankQuery> = QueryRequest::Bank(BankQuery::Balance {
                address: ADMIN.to_string(),
                denom: NATIVE_DENOM_2.to_string(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let balance: BankBalanceResponse = from_binary(&res).unwrap();

            assert_eq!(
                balance.amount.amount,
                Uint128::from(
                    INIT_1000_000_NATIVE_BALANCE_2 - ADD_1000_NATIVE_BALANCE_2
                        + pending_reward_admin_2s.amount.u128()
                        + pending_reward_admin_6s.amount.u128()
                )
            );

            // change block time increase 2 seconds to make 8 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(2),
                height: app.block_info().height + 2,
                chain_id: app.block_info().chain_id,
            });

            // Query pending reward by USER_1
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: USER_1.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_user1_8s: RewardTokenAsset = from_binary(&res).unwrap();

            // It should be 23333333 as reward is accrued
            assert_eq!(
                pending_reward_user1_8s,
                RewardTokenAsset {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(23_333_333u128)
                }
            );

            // Withdraw 100% lp token from the pool contract by USER_1
            let withdraw_msg = PoolExecuteMsg::Withdraw {
                amount: Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT / 2),
            };

            // Execute withdraw by USER_1
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &withdraw_msg,
                &[],
            );

            assert!(response.is_ok());

            // query balance of USER_1 in cw20 base token contract
            let balance: BalanceResponse = app
                .wrap()
                .query_wasm_smart(
                    lp_token_contract.clone(),
                    &cw20::Cw20QueryMsg::Balance {
                        address: USER_1.to_string(),
                    },
                )
                .unwrap();

            // It should be 500 lp token
            assert_eq!(
                balance.balance,
                Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT / 2)
            );

            // query balance of USER_1 in native token
            let req: QueryRequest<BankQuery> = QueryRequest::Bank(BankQuery::Balance {
                address: USER_1.to_string(),
                denom: NATIVE_DENOM_2.to_string(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let balance: BankBalanceResponse = from_binary(&res).unwrap();

            assert_eq!(
                balance.amount.amount,
                Uint128::from(
                    pending_reward_user1_2s.amount.u128() + pending_reward_user1_8s.amount.u128()
                )
            );

            // change block time increase 2 seconds to make 10 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(2),
                height: app.block_info().height + 2,
                chain_id: app.block_info().chain_id,
            });

            // Query pending reward by ADMIN
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_admin_10s: RewardTokenAsset = from_binary(&res).unwrap();

            // It should be 30000000 as reward is accrued
            assert_eq!(
                pending_reward_admin_10s,
                RewardTokenAsset {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(30_000_000u128)
                }
            );

            // Harvest reward by ADMIN
            let harvest_msg = PoolExecuteMsg::Harvest {};

            // Execute harvest by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &harvest_msg,
                &[],
            );

            assert!(response.is_ok());

            // query balance of ADMIN in native token
            let req: QueryRequest<BankQuery> = QueryRequest::Bank(BankQuery::Balance {
                address: ADMIN.to_string(),
                denom: NATIVE_DENOM_2.to_string(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let balance: BankBalanceResponse = from_binary(&res).unwrap();

            assert_eq!(
                balance.amount.amount,
                Uint128::from(
                    INIT_1000_000_NATIVE_BALANCE_2 - ADD_1000_NATIVE_BALANCE_2
                        + pending_reward_admin_2s.amount.u128()
                        + pending_reward_admin_6s.amount.u128()
                        + pending_reward_admin_10s.amount.u128()
                )
            );

            // change block time increase 2 seconds to make 12 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(2),
                height: app.block_info().height + 2,
                chain_id: app.block_info().chain_id,
            });

            // Query pending reward by USER_1
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: USER_1.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_user_1_10s: RewardTokenAsset = from_binary(&res).unwrap();

            // It should be 0 as all lp token is withdrawn
            assert_eq!(
                pending_reward_user_1_10s,
                RewardTokenAsset {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::zero()
                }
            );

            // Harvest reward by USER_1
            let harvest_msg = PoolExecuteMsg::Harvest {};

            // Execute harvest by USER_1
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &harvest_msg,
                &[],
            );

            assert_eq!(
                response.unwrap_err().source().unwrap().to_string(),
                "Generic error: Unauthorized: Only staker can harvest reward".to_string()
            );

            // Mint 500 HALO LP tokens to ADMIN
            let mint_msg: Cw20ExecuteMsg = Cw20ExecuteMsg::Mint {
                recipient: ADMIN.to_string(),
                amount: Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT / 2),
            };

            // Execute minting
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(lp_token_contract.clone()),
                &mint_msg,
                &[],
            );

            assert!(response.is_ok());

            // Approve cw20 token to pool contract
            let approve_msg: Cw20ExecuteMsg = Cw20ExecuteMsg::IncreaseAllowance {
                spender: "contract3".to_string(), // Pool Contract
                amount: Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT / 2),
                expires: None,
            };

            // Execute approve by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(lp_token_contract.clone()),
                &approve_msg,
                &[],
            );

            assert!(response.is_ok());

            // change block time increase 2 seconds to make 14 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(2),
                height: app.block_info().height + 2,
                chain_id: app.block_info().chain_id,
            });

            // Query pending reward by ADMIN after 14 seconds
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_admin_14s: RewardTokenAsset = from_binary(&res).unwrap();

            // It should be 40000000 as reward is accrued
            assert_eq!(
                pending_reward_admin_14s,
                RewardTokenAsset {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(40_000_000u128)
                }
            );

            // Deposit lp token to the pool contract to execute deposit msg
            let deposit_msg = PoolExecuteMsg::Deposit {
                amount: Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT / 2),
            };

            // Execute deposit by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &deposit_msg,
                &[],
            );

            assert!(response.is_ok());

            // Query NATIVE_DENOM_2 balance of ADMIN
            let req: QueryRequest<BankQuery> = QueryRequest::Bank(BankQuery::Balance {
                address: ADMIN.to_string(),
                denom: NATIVE_DENOM_2.to_string(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let balance: BankBalanceResponse = from_binary(&res).unwrap();

            assert_eq!(
                balance.amount.amount,
                Uint128::from(
                    INIT_1000_000_NATIVE_BALANCE_2 - ADD_1000_NATIVE_BALANCE_2
                        + pending_reward_admin_2s.amount.u128()
                        + pending_reward_admin_6s.amount.u128()
                        + pending_reward_admin_10s.amount.u128()
                        + pending_reward_admin_14s.amount.u128()
                )
            );

            // change block time increase 2 seconds to make 16 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(2),
                height: app.block_info().height + 2,
                chain_id: app.block_info().chain_id,
            });

            // Deposit 150 lp token to the pool contract by USER_1
            let deposit_msg = PoolExecuteMsg::Deposit {
                amount: Uint128::from(MOCK_150_HALO_LP_TOKEN_AMOUNT),
            };

            // Execute deposit by USER_1
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &deposit_msg,
                &[],
            );

            assert!(response.is_ok());

            // Query pending reward by ADMIN after 16 seconds
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_admin_16s: RewardTokenAsset = from_binary(&res).unwrap();

            // It should be 20000000 as reward is accrued
            assert_eq!(
                pending_reward_admin_16s,
                RewardTokenAsset {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(20_000_000u128)
                }
            );

            // change block time increase 2 seconds to make 18 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(2),
                height: app.block_info().height + 2,
                chain_id: app.block_info().chain_id,
            });

            // Query pending reward by ADMIN after 18 seconds
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_admin_18s: RewardTokenAsset = from_binary(&res).unwrap();

            // It should be 37391305 as reward is accrued
            assert_eq!(
                pending_reward_admin_18s,
                RewardTokenAsset {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(37_391_305u128)
                }
            );

            // Harvest reward by ADMIN
            let harvest_msg = PoolExecuteMsg::Harvest {};

            // Execute harvest by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &harvest_msg,
                &[],
            );

            assert!(response.is_ok());

            // query balance of ADMIN in native token
            let req: QueryRequest<BankQuery> = QueryRequest::Bank(BankQuery::Balance {
                address: ADMIN.to_string(),
                denom: NATIVE_DENOM_2.to_string(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let balance: BankBalanceResponse = from_binary(&res).unwrap();

            // It should be 999_147_391_304 as reward is accrued
            assert_eq!(
                Uint128::from(999_147_391_304u128),
                balance.amount.amount,
            );
            assert_eq!(
                balance.amount.amount,
                Uint128::from(
                    INIT_1000_000_NATIVE_BALANCE_2 - ADD_1000_NATIVE_BALANCE_2
                        + pending_reward_admin_2s.amount.u128()
                        + pending_reward_admin_6s.amount.u128()
                        + pending_reward_admin_10s.amount.u128()
                        + pending_reward_admin_14s.amount.u128()
                        // + pending_reward_admin_16s.amount.u128() // Did not executed harvest
                        + pending_reward_admin_18s.amount.u128() // Included pending_reward_admin_16s
                )
            );

            // Query pending reward by USER_1 after 18 seconds
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: USER_1.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_user_1_18s: RewardTokenAsset = from_binary(&res).unwrap();

            // It should be 2608696 as reward is accrued
            assert_eq!(
                pending_reward_user_1_18s,
                RewardTokenAsset {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(2_608_696u128)
                }
            );

            // Harvest reward by USER_1
            let harvest_msg = PoolExecuteMsg::Harvest {};

            // Execute harvest by USER_1
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &harvest_msg,
                &[],
            );

            assert!(response.is_ok());

            // query balance of USER_1 in native token
            let req: QueryRequest<BankQuery> = QueryRequest::Bank(BankQuery::Balance {
                address: USER_1.to_string(),
                denom: NATIVE_DENOM_2.to_string(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let balance: BankBalanceResponse = from_binary(&res).unwrap();

            // It should be 32_608_695 as reward is accrued
            assert_eq!(
                Uint128::from(32_608_695u128),
                balance.amount.amount,
            );
            assert_eq!(
                balance.amount.amount,
                Uint128::from(
                    pending_reward_user1_2s.amount.u128()
                        + pending_reward_user1_8s.amount.u128()
                        + pending_reward_user_1_18s.amount.u128()
                )
            );

            // Extend end time by ADMIN more 80 seconds
            let extend_end_time_msg = PoolExecuteMsg::AddPhase {
                new_start_time: pool_info.end_time + 10,
                new_end_time: pool_info.end_time + 90,
            };

            // Execute extend end time by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &extend_end_time_msg,
                &[],
            );

            assert!(response.is_ok());

            // change block time increase 82 seconds to make 100 seconds passed (end time)
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(82),
                height: app.block_info().height + 82,
                chain_id: app.block_info().chain_id,
            });

            // Query staked info of ADMIN
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::StakedInfo {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let staked_info_admin: StakerRewardAssetInfo = from_binary(&res).unwrap();

            assert_eq!(
                staked_info_admin,
                StakerRewardAssetInfo {
                    amount: Uint128::from(ADD_1000_NATIVE_BALANCE_2),
                    reward_debt: Uint128::from(217_391_304u128),
                    joined_phases: 0u64,
                }
            );

            // Query pending reward by ADMIN after 100 seconds (end time)
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_admin_100s: RewardTokenAsset = from_binary(&res).unwrap();

            // It should be 713_043_478 as reward is accrued
            assert_eq!(
                pending_reward_admin_100s,
                RewardTokenAsset {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(713_043_478u128)
                }
            );

            // Harvest reward by ADMIN
            let harvest_msg = PoolExecuteMsg::Harvest {};

            // Execute harvest by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &harvest_msg,
                &[],
            );

            assert!(response.is_ok());

            // query balance of ADMIN in native token
            let req: QueryRequest<BankQuery> = QueryRequest::Bank(BankQuery::Balance {
                address: ADMIN.to_string(),
                denom: NATIVE_DENOM_2.to_string(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let balance: BankBalanceResponse = from_binary(&res).unwrap();

            // It should be 999_860_434_782 as reward is accrued
            assert_eq!(
                Uint128::from(999_860_434_782u128),
                balance.amount.amount,
            );
            assert_eq!(
                balance.amount.amount,
                Uint128::from(
                    INIT_1000_000_NATIVE_BALANCE_2 - ADD_1000_NATIVE_BALANCE_2
                        + pending_reward_admin_2s.amount.u128()
                        + pending_reward_admin_6s.amount.u128()
                        + pending_reward_admin_10s.amount.u128()
                        + pending_reward_admin_14s.amount.u128()
                        // + pending_reward_admin_16s.amount.u128() // not execute harvest yet
                        + pending_reward_admin_18s.amount.u128() // Included pending_reward_admin_16s
                        + pending_reward_admin_100s.amount.u128()
                )
            );

            // Query pending reward by USER_1 after 100 seconds (end time)
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: USER_1.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_user_1_100s: RewardTokenAsset = from_binary(&res).unwrap();

            // It should be 106_956_522 as reward is accrued
            assert_eq!(
                pending_reward_user_1_100s,
                RewardTokenAsset {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(106_956_522u128)
                }
            );

            // Query USER_1 balance in native token
            let req: QueryRequest<BankQuery> = QueryRequest::Bank(BankQuery::Balance {
                address: USER_1.to_string(),
                denom: NATIVE_DENOM_2.to_string(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let balance: BankBalanceResponse = from_binary(&res).unwrap();

            // It should be 32_608_695 as reward is accrued
            assert_eq!(
                Uint128::from(32_608_695u128),
                balance.amount.amount,
            );

            let reward_asset_info = TokenInfo::NativeToken {
                denom: NATIVE_DENOM_2.to_string(),
            };

            // Add 1000 NATIVE_DENOM_2 reward balance amount to pool contract by ADMIN
            let add_reward_balance_msg = PoolExecuteMsg::AddRewardBalance {
                phase_index: 1u64,
                asset: RewardTokenAsset {
                    info: reward_asset_info,
                    amount: Uint128::from(ADD_1000_NATIVE_BALANCE_2),
                },
            };

            // Execute add reward balance by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &add_reward_balance_msg,
                &[Coin {
                    amount: Uint128::from(ADD_1000_NATIVE_BALANCE_2),
                    denom: NATIVE_DENOM_2.to_string(),
                }],
            );

            assert!(response.is_ok());

            // change block time increase 5 seconds to make 105 seconds passed to activate new phase
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(5),
                height: app.block_info().height + 5,
                chain_id: app.block_info().chain_id,
            });

            // Activate new phase
            let activate_phase_msg = PoolExecuteMsg::ActivatePhase {};

            // Execute activate phase by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &activate_phase_msg,
                &[],
            );

            assert!(response.is_ok());

            // Query pool info after add reward balance
            let pool_info_phase1: PoolInfo = app
                .wrap()
                .query_wasm_smart("contract3", &PoolQueryMsg::Pool {})
                .unwrap();

            // assert pool info
            assert_eq!(
                pool_info_phase1,
                PoolInfo {
                    staked_token: lp_token_contract.to_string(),
                    reward_token: native_token_info,
                    reward_per_second: Decimal::from_str("12500000").unwrap(), // 12_500_000 (12.5 NATIVE_DENOM_2)
                    start_time: pool_info.end_time + 10,
                    end_time: pool_info.end_time + 90,
                    pool_limit_per_user: None,
                    whitelist: vec![Addr::unchecked(ADMIN.to_string())],
                }
            );

            // change block time increase 25 seconds to make 135 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(25),
                height: app.block_info().height + 25,
                chain_id: app.block_info().chain_id,
            });

            // Query pending reward by ADMIN after 135 seconds
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_admin_135s: RewardTokenAsset = from_binary(&res).unwrap();

            // It should be 217_391_304 as reward is accrued
            assert_eq!(
                pending_reward_admin_135s,
                RewardTokenAsset {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(217_391_304u128)
                }
            );

            // Harvest reward by ADMIN
            let harvest_msg = PoolExecuteMsg::Harvest {};

            // Execute harvest by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &harvest_msg,
                &[],
            );

            assert!(response.is_ok());

            // Query staked info of ADMIN after join new phase
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::StakedInfo {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let staked_info_admin: StakerRewardAssetInfo = from_binary(&res).unwrap();

            assert_eq!(
                staked_info_admin,
                StakerRewardAssetInfo {
                    amount: Uint128::from(ADD_1000_NATIVE_BALANCE_2),
                    reward_debt: Uint128::from(217_391_304u128),
                    joined_phases: 1u64, // Joined new phases
                }
            );

            // query balance of ADMIN in native token
            let req: QueryRequest<BankQuery> = QueryRequest::Bank(BankQuery::Balance {
                address: ADMIN.to_string(),
                denom: NATIVE_DENOM_2.to_string(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let balance: BankBalanceResponse = from_binary(&res).unwrap();

            // It should be 999_077_826_086 as reward is accrued
            assert_eq!(
                balance.amount.amount,
                Uint128::from(
                    INIT_1000_000_NATIVE_BALANCE_2 - ADD_1000_NATIVE_BALANCE_2 - ADD_1000_NATIVE_BALANCE_2
                        + pending_reward_admin_2s.amount.u128()
                        + pending_reward_admin_6s.amount.u128()
                        + pending_reward_admin_10s.amount.u128()
                        + pending_reward_admin_14s.amount.u128()
                        // + pending_reward_admin_16s.amount.u128() // not execute harvest yet
                        + pending_reward_admin_18s.amount.u128() // Included pending_reward_admin_16s
                        + pending_reward_admin_100s.amount.u128()
                        + pending_reward_admin_135s.amount.u128()
                )
            );

            assert_eq!(
                balance.amount.amount,
                Uint128::from(999_077_826_086u128),
            );

            // Query pending reward by USER_1 after 135 seconds
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: USER_1.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_user_1_135s: RewardTokenAsset = from_binary(&res).unwrap();

            // It should be 106_956_522 + 32_608_695 = 139_565_217 as reward is accrued
            assert_eq!(
                pending_reward_user_1_135s,
                RewardTokenAsset {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(139_565_217u128)
                }
            );

            // Harvest reward by USER_1
            let harvest_msg = PoolExecuteMsg::Harvest {};

            // Execute harvest by USER_1
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &harvest_msg,
                &[],
            );

            assert!(response.is_ok());

            // query balance of USER_1 in native token
            let req: QueryRequest<BankQuery> = QueryRequest::Bank(BankQuery::Balance {
                address: USER_1.to_string(),
                denom: NATIVE_DENOM_2.to_string(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let balance: BankBalanceResponse = from_binary(&res).unwrap();

            // It should be 106_956_522 + 32_608_695 = 139_565_217 as reward is accrued
            assert_eq!(
                balance.amount.amount,
                Uint128::from(
                    pending_reward_user1_2s.amount.u128()
                        + pending_reward_user1_8s.amount.u128()
                        + pending_reward_user_1_18s.amount.u128()
                        // + pending_reward_user_1_100s.amount.u128()
                        + pending_reward_user_1_135s.amount.u128() // Included pending_reward_user_1_100s
                )
            );


        }

/*
        // Mint 1000 HALO LP token for ADMIN
        // Mint 500 HALO LP token for USER_1
        // Mint 1000 HALO REWARD token for ADMIN
        // Create pool contract by factory contract
        // Add 1000 HALO REWARD token reward balance to pool contract by ADMIN
        // with end time 100 seconds
        // -> 10 HALO REWARD token per second
        // Deposit 1000 HALO LP token to the pool contract by ADMIN
        //
        // Harvest reward by ADMIN after 2 seconds
        // -> 2s: 20 HALO REWARD token for ADMIN
        //
        // USER_1 deposit 500 HALO LP token to the pool contract
        // Harvest reward by USER_1 after 4 seconds (1)
        // -> 2s: 6,6666 HALO REWARD token for USER_1
        //
        // Withdraw 500 HALO LP token from the pool contract by ADMIN after 6 seconds
        // -> 2s(1) + 2s: 13,33 + 13,33 = 26,66 HALO REWARD token for ADMIN
        //
        // Increase 1 second to make 7 seconds passed
        // -> 1s: 5 HALO REWARD token for ADMIN (2)
        // Harvest reward by ADMIN after 8 seconds
        // -> 1s(2) + 1s = 5 + 6,666 = 11,666 HALO REWARD token for ADMIN
        #[test]
        fn proper_operation_with_reward_token_decimal_18() {
            // get integration test app and contracts
            let (mut app, contracts) = instantiate_contracts();
            // ADMIN already has 1_000_000 NATIVE_DENOM_2 as initial balance in instantiate_contracts()
            // get pool factory contract
            let factory_contract = &contracts[0].contract_addr;
            // get halo lp token contract
            let lp_token_contract = &contracts[1].contract_addr;
            // get halo reward token contract
            let reward_token_contract = &contracts[2].contract_addr;

            // get current block time
            let current_block_time = app.block_info().time.seconds();

            // Mint 1000 HALO LP tokens to ADMIN
            let mint_msg: Cw20ExecuteMsg = Cw20ExecuteMsg::Mint {
                recipient: ADMIN.to_string(),
                amount: Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT),
            };

            // Execute minting
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(lp_token_contract.clone()),
                &mint_msg,
                &[],
            );

            assert!(response.is_ok());

            // Mint 500 HALO LP tokens to USER_1
            let mint_msg: Cw20ExecuteMsg = Cw20ExecuteMsg::Mint {
                recipient: USER_1.to_string(),
                amount: Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT / 2),
            };

            // Execute minting
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(lp_token_contract.clone()),
                &mint_msg,
                &[],
            );

            assert!(response.is_ok());

            // Mint 1000 HALO reward tokens to ADMIN
            let mint_msg: Cw20ExecuteMsg = Cw20ExecuteMsg::Mint {
                recipient: ADMIN.to_string(),
                amount: Uint128::from(MOCK_1000_HALO_REWARD_TOKEN_AMOUNT),
            };

            // Execute minting
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(reward_token_contract.clone()),
                &mint_msg,
                &[],
            );

            assert!(response.is_ok());

            // reward token info
            let reward_token_info = TokenInfo::Token {
                contract_addr: reward_token_contract.clone(),
            };

            // create pool contract by factory contract
            let create_pool_msg = crate::msg::ExecuteMsg::CreatePool {
                staked_token: lp_token_contract.clone(),
                reward_token: reward_token_info.clone(),
                start_time: current_block_time,
                end_time: current_block_time + 100,
                pool_limit_per_user: None,
                whitelist: vec![Addr::unchecked(ADMIN.to_string())],
            };

            // Execute create pool by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(factory_contract.clone()),
                &create_pool_msg,
                &[],
            );

            assert!(response.is_ok());

            // Increase allowance of reward token to pool contract
            let approve_msg: Cw20ExecuteMsg = Cw20ExecuteMsg::IncreaseAllowance {
                spender: "contract3".to_string(), // Pool Contract
                amount: Uint128::from(MOCK_1000_HALO_REWARD_TOKEN_AMOUNT),
                expires: None,
            };

            // Execute approve by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(reward_token_contract.clone()),
                &approve_msg,
                &[],
            );

            assert!(response.is_ok());

            // add 1000 reward balance to pool contract
            let add_reward_balance_msg = PoolExecuteMsg::AddRewardBalance {
                phase_index: 0u64,
                asset: RewardTokenAsset {
                    info: reward_token_info,
                    amount: Uint128::from(MOCK_1000_HALO_REWARD_TOKEN_AMOUNT),
                },
            };

            // Execute add reward by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &add_reward_balance_msg,
                &[],
            );

            assert!(response.is_ok());

            // Increase allowance of lp token to pool contract
            let approve_msg: Cw20ExecuteMsg = Cw20ExecuteMsg::IncreaseAllowance {
                spender: "contract3".to_string(), // Pool Contract
                amount: Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT),
                expires: None,
            };

            // Execute approve by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(lp_token_contract.clone()),
                &approve_msg,
                &[],
            );

            assert!(response.is_ok());

            // Deposit lp token to the pool contract to execute deposit msg
            let deposit_msg = PoolExecuteMsg::Deposit {
                amount: Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT),
            };

            // Execute deposit by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &deposit_msg,
                &[],
            );

            assert!(response.is_ok());

            // change block time increase 2 seconds to make 2 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(2),
                height: app.block_info().height + 1,
                chain_id: app.block_info().chain_id,
            });

            // Query pending reward by ADMIN after 2 seconds
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_admin_2s: RewardTokenAsset = from_binary(&res).unwrap();

            // It should be 20x10^18 as reward is accrued
            assert_eq!(
                pending_reward_admin_2s,
                RewardTokenAsset {
                    info: TokenInfo::Token {
                        contract_addr: reward_token_contract.clone()
                    },
                    amount: Uint128::from(20_000_000_000_000_000_000u128)
                }
            );

            // Harvest reward by ADMIN
            let harvest_msg = PoolExecuteMsg::Harvest {};

            // Execute harvest by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &harvest_msg,
                &[],
            );

            assert!(response.is_ok());

            // query balance of ADMIN in reward token
            let balance: BalanceResponse = app
                .wrap()
                .query_wasm_smart(
                    reward_token_contract.clone(),
                    &cw20::Cw20QueryMsg::Balance {
                        address: ADMIN.to_string(),
                    },
                )
                .unwrap();

            // It should be 20x10^18 reward token
            assert_eq!(
                balance.balance,
                Uint128::from(20_000_000_000_000_000_000u128)
            );

            // Increase allowance of lp token to pool contract
            let approve_msg: Cw20ExecuteMsg = Cw20ExecuteMsg::IncreaseAllowance {
                spender: "contract3".to_string(), // Pool Contract
                amount: Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT / 2),
                expires: None,
            };

            // Execute approve by USER_1
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked(lp_token_contract.clone()),
                &approve_msg,
                &[],
            );

            assert!(response.is_ok());

            // USER_1 deposit 500 HALO LP token to the pool contract
            let deposit_msg = PoolExecuteMsg::Deposit {
                amount: Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT / 2),
            };

            // Execute deposit by USER_1
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &deposit_msg,
                &[],
            );

            assert!(response.is_ok());

            // change block time increase 2 seconds to make 4 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(2),
                height: app.block_info().height + 2,
                chain_id: app.block_info().chain_id,
            });

            // Query pending reward by USER_1 after 4 seconds
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: USER_1.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_user1_4s: RewardTokenAsset = from_binary(&res).unwrap();

            // It should be 6,6666x10^18 as reward is accrued
            assert_eq!(
                pending_reward_user1_4s,
                RewardTokenAsset {
                    info: TokenInfo::Token {
                        contract_addr: reward_token_contract.clone()
                    },
                    amount: Uint128::from(6_666_666_666_666_666_666u128)
                }
            );

            // Harvest reward by USER_1
            let harvest_msg = PoolExecuteMsg::Harvest {};

            // Execute harvest by USER_1
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &harvest_msg,
                &[],
            );

            assert!(response.is_ok());

            // query balance of USER_1 in reward token
            let balance: BalanceResponse = app
                .wrap()
                .query_wasm_smart(
                    reward_token_contract.clone(),
                    &cw20::Cw20QueryMsg::Balance {
                        address: USER_1.to_string(),
                    },
                )
                .unwrap();

            // It should be 6,6666x10^18 reward token
            assert_eq!(balance.balance, pending_reward_user1_4s.amount);

            // change block time increase 2 seconds to make 6 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(2),
                height: app.block_info().height + 2,
                chain_id: app.block_info().chain_id,
            });

            // query pending reward by ADMIN after 6 seconds
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_admin_6s: RewardTokenAsset = from_binary(&res).unwrap();

            // It should be 26,666x10^18 as reward is accrued
            assert_eq!(
                pending_reward_admin_6s,
                RewardTokenAsset {
                    info: TokenInfo::Token {
                        contract_addr: reward_token_contract.clone()
                    },
                    amount: Uint128::from(26_666_666_666_666_666_666u128)
                }
            );

            // Withdraw 500 HALO LP token from the pool contract by ADMIN
            let withdraw_msg = PoolExecuteMsg::Withdraw {
                amount: Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT / 2),
            };

            // Execute withdraw by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &withdraw_msg,
                &[],
            );

            assert!(response.is_ok());

            // query balance of ADMIN in reward token

            let balance: BalanceResponse = app
                .wrap()
                .query_wasm_smart(
                    reward_token_contract.clone(),
                    &cw20::Cw20QueryMsg::Balance {
                        address: ADMIN.to_string(),
                    },
                )
                .unwrap();

            // It should be 46,6666x10^18 reward token
            assert_eq!(
                balance.balance,
                pending_reward_admin_2s.amount + pending_reward_admin_6s.amount
            );

            // change block time increase 1 seconds to make 7 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(1),
                height: app.block_info().height + 1,
                chain_id: app.block_info().chain_id,
            });

            // query pending reward by ADMIN after 7 seconds
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_admin_7s: RewardTokenAsset = from_binary(&res).unwrap();

            // It should be 5x10^18 as reward is accrued
            assert_eq!(
                pending_reward_admin_7s,
                RewardTokenAsset {
                    info: TokenInfo::Token {
                        contract_addr: reward_token_contract.clone()
                    },
                    amount: Uint128::from(5_000_000_000_000_000_000u128)
                }
            );

            // Increase allowance of lp token to pool contract
            let approve_msg: Cw20ExecuteMsg = Cw20ExecuteMsg::IncreaseAllowance {
                spender: "contract3".to_string(), // Pool Contract
                amount: Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT / 2),
                expires: None,
            };

            // Execute approve by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(lp_token_contract.clone()),
                &approve_msg,
                &[],
            );

            assert!(response.is_ok());

            // deposit 500 HALO LP token to the pool contract by ADMIN
            let deposit_msg = PoolExecuteMsg::Deposit {
                amount: Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT / 2),
            };

            // Execute deposit by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &deposit_msg,
                &[],
            );

            assert!(response.is_ok());

            // change block time increase 1 seconds to make 8 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(1),
                height: app.block_info().height + 1,
                chain_id: app.block_info().chain_id,
            });

            // query pending reward by ADMIN after 8 seconds
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_admin_8s: RewardTokenAsset = from_binary(&res).unwrap();

            // It should be 6,66x10^18 as reward is accrued
            assert_eq!(
                pending_reward_admin_8s,
                RewardTokenAsset {
                    info: TokenInfo::Token {
                        contract_addr: reward_token_contract.clone()
                    },
                    amount: Uint128::from(6_666_666_666_666_666_667u128)
                }
            );

            // Harvest reward by ADMIN
            let harvest_msg = PoolExecuteMsg::Harvest {};

            // Execute harvest by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &harvest_msg,
                &[],
            );

            assert!(response.is_ok());

            // query balance of ADMIN in reward token
            let balance: BalanceResponse = app
                .wrap()
                .query_wasm_smart(
                    reward_token_contract.clone(),
                    &cw20::Cw20QueryMsg::Balance {
                        address: ADMIN.to_string(),
                    },
                )
                .unwrap();

            // It should be 53,3333x10^18 reward token
            assert_eq!(
                balance.balance,
                pending_reward_admin_2s.amount
                    + pending_reward_admin_6s.amount
                    + pending_reward_admin_7s.amount
                    + pending_reward_admin_8s.amount
            );
        }
         */
    }
}
