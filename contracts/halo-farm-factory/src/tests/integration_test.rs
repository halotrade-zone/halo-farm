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
    mod execute_proper_operation {
        use std::str::FromStr;

        use cosmwasm_std::{
            from_binary, to_binary, Addr, BalanceResponse as BankBalanceResponse, BankQuery,
            BlockInfo, Coin, Decimal, Querier, QueryRequest, Uint128, WasmQuery,
        };
        use cw20::{BalanceResponse, Cw20ExecuteMsg};
        use cw_multi_test::Executor;
        use halo_farm::state::{
            PhaseInfo, PoolInfos, RewardTokenAssetResponse, StakerInfoResponse, TokenInfo,
        };

        use crate::{
            msg::QueryMsg,
            state::FactoryPoolInfo,
            tests::{
                env_setup::env::{
                    instantiate_contracts, ADMIN, NATIVE_BALANCE_2, NATIVE_DENOM_2, USER_1,
                },
                integration_test::tests::{
                    ADD_1000_NATIVE_BALANCE_2, INIT_1000_000_NATIVE_BALANCE_2,
                    MOCK_1000_HALO_LP_TOKEN_AMOUNT, MOCK_1000_HALO_REWARD_TOKEN_AMOUNT,
                    MOCK_150_HALO_LP_TOKEN_AMOUNT,
                },
            },
        };
        use halo_farm::msg::{ExecuteMsg as PoolExecuteMsg, QueryMsg as PoolQueryMsg};

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
                staked_token: Addr::unchecked(lp_token_contract.clone()),
                reward_token: native_token_info.clone(),
                start_time: current_block_time,
                end_time: current_block_time + 100,
                pool_limit_per_user: None,
                whitelist: Addr::unchecked(ADMIN.to_string()),
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
                    staked_token: Addr::unchecked(lp_token_contract),
                    reward_token: native_token_info.clone(),
                    start_time: current_block_time,
                    end_time: current_block_time + 100,
                    pool_limit_per_user: None,
                }
            );

            // add reward balance to pool contract
            let add_reward_balance_msg = PoolExecuteMsg::AddRewardBalance {
                phase_index: 0u64,
                amount: Uint128::from(ADD_1000_NATIVE_BALANCE_2),
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
            let pool_info: PoolInfos = app
                .wrap()
                .query_wasm_smart("contract3", &PoolQueryMsg::Pool {})
                .unwrap();

            // assert pool info
            assert_eq!(
                pool_info,
                PoolInfos {
                    staked_token: Addr::unchecked(lp_token_contract.clone()),
                    reward_token: native_token_info.clone(),
                    current_phase_index: 0u64,
                    phases_info: vec![PhaseInfo {
                        start_time: current_block_time,
                        end_time: current_block_time + 100,
                        whitelist: Addr::unchecked(ADMIN.to_string()),
                        reward_balance: Uint128::from(ADD_1000_NATIVE_BALANCE_2),
                        last_reward_time: current_block_time,
                        accrued_token_per_share: Decimal::zero(),
                        total_staked_at_end_time: Uint128::zero(),
                    }],
                    pool_limit_per_user: None,
                    staked_token_balance: Uint128::zero(),
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
                    staked_token: Addr::unchecked(lp_token_contract),
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

            // increase 1 second to make phase active
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(1),
                height: app.block_info().height + 1,
                chain_id: app.block_info().chain_id,
            });

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
            let pending_reward: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 60_000_000 as reward is accrued
            assert_eq!(
                pending_reward,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(60_000_000u128),
                    time_query: app.block_info().time.seconds(),
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
            let pending_reward: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 20_000_000 as reward is accrued
            assert_eq!(
                pending_reward,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(20_000_000u128),
                    time_query: 1571797428,
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
        // Harvest reward by ADMIN after 135 seconds
        //   -> Reward balance: 25s: (1000 / (1000 + 150)) * (25-5) * 12.5  = 217,391 NATIVE_2
        // Harvest reward by USER_1 after 25 seconds
        //   -> Reward balance: 25s: (150 / (1000 + 150)) * (25-5) * 12.5  = 32,608 NATIVE_2
        //                      100s in Phase 0: (Not claim yet) = 106,956 NATIVE_2
        //                      = 139,564 NATIVE_2
        // Harvest reward by ADMIN after 150 seconds
        //   -> Reward balance: 15s: (1000 / (1000 + 150)) * 15 * 12.5  = 163,043 NATIVE_2
        // Harvest reward by USER_1 after 150 seconds
        //   -> Reward balance: 15s: (150 / (1000 + 150)) * 15 * 12.5  = 24,456 NATIVE_2
        // Withdraw 50% ADMIN's staked lp token from the pool contract by ADMIN after 155 seconds
        //   -> ADMIN Lp token balance in pool: 500 LP token
        //   -> Reward balance: 5s: (1000 / (1000 + 150)) * 5 * 12.5  = 54,347 NATIVE_2
        // USER_1 Harvest reward after 160 seconds
        //   -> Reward balance: 5s: (150 / (1000 + 150)) * 5 * 12.5  = 8,152 NATIVE_2
        //                      10s: (150 / (500 + 150)) * 5 * 12.5  = 14,423 NATIVE_2
        //                                                           = 22,575 NATIVE_2
        // ADMIN Deposit 500 lp token to the pool contract after 165 seconds
        //   -> ADMIN Lp token balance in pool: 1000 LP token
        //   -> Reward balance: 5s: (500 / (500 + 150)) * 5 * 12.5  = 48,076 NATIVE_2
        //                     10s: (500 / (500 + 150)) * 5 * 12.5  = 48,076 NATIVE_2
        //                                                          = 96,153 NATIVE_2 (Not claim yet)
        // Increase simulation time more 5 seconds
        // ADMIN Harvest reward after 170 seconds
        //   -> Reward balance: 10s:                                  = 96,153 NATIVE_2 (Not claim yet)
        //                      5s: (1000 / (1000 + 150)) * 5 * 12.5  = 54,347 NATIVE_2
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
                staked_token: Addr::unchecked(lp_token_contract.clone()),
                reward_token: native_token_info.clone(),
                start_time: current_block_time,
                end_time: current_block_time + 100,
                pool_limit_per_user: None,
                whitelist: Addr::unchecked(ADMIN.to_string()),
            };

            // Execute create pool
            let response_create_pool = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(factory_contract.clone()),
                &create_pool_msg,
                &[],
            );

            assert!(response_create_pool.is_ok());

            // add reward balance to pool contract
            let add_reward_balance_msg = PoolExecuteMsg::AddRewardBalance {
                phase_index: 0u64,
                amount: Uint128::from(ADD_1000_NATIVE_BALANCE_2),
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
            let pool_info: PoolInfos = app
                .wrap()
                .query_wasm_smart("contract3", &PoolQueryMsg::Pool {})
                .unwrap();

            // assert pool info
            assert_eq!(
                pool_info,
                PoolInfos {
                    staked_token: Addr::unchecked(lp_token_contract),
                    reward_token: native_token_info.clone(),
                    current_phase_index: 0u64,
                    phases_info: vec![PhaseInfo {
                        start_time: current_block_time,
                        end_time: current_block_time + 100,
                        whitelist: Addr::unchecked(ADMIN.to_string()),
                        reward_balance: Uint128::from(ADD_1000_NATIVE_BALANCE_2),
                        last_reward_time: current_block_time,
                        accrued_token_per_share: Decimal::zero(),
                        total_staked_at_end_time: Uint128::zero(),
                    }],
                    pool_limit_per_user: None,
                    staked_token_balance: Uint128::zero(),
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
            let pending_reward_admin_2s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 13333333 as reward is accrued
            assert_eq!(
                pending_reward_admin_2s,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(13_333_333u128),
                    time_query: 1571797421
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
            let pending_reward_user1_2s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 6666666 as reward is accrued
            assert_eq!(
                pending_reward_user1_2s,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(6_666_666u128),
                    time_query: 1571797421
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
            let pending_reward_admin_6s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 26666666 as reward is accrued
            assert_eq!(
                pending_reward_admin_6s,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(26_666_666u128),
                    time_query: 1571797425
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
            let pending_reward_user1_8s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 23333333 as reward is accrued
            assert_eq!(
                pending_reward_user1_8s,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(23_333_333u128),
                    time_query: 1571797427
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
            let pending_reward_admin_10s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 30000000 as reward is accrued
            assert_eq!(
                pending_reward_admin_10s,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(30_000_000u128),
                    time_query: 1571797429
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
            let pending_reward_user_1_10s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 0 as all lp token is withdrawn
            assert_eq!(
                pending_reward_user_1_10s,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::zero(),
                    time_query: 1571797431
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
            let pending_reward_admin_14s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 40000000 as reward is accrued
            assert_eq!(
                pending_reward_admin_14s,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(40_000_000u128),
                    time_query: 1571797433
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
            let pending_reward_admin_16s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 20000000 as reward is accrued
            assert_eq!(
                pending_reward_admin_16s,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(20_000_000u128),
                    time_query: 1571797435
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
            let pending_reward_admin_18s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 37391305 as reward is accrued
            assert_eq!(
                pending_reward_admin_18s,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(37_391_305u128),
                    time_query: 1571797437
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
            assert_eq!(Uint128::from(999_147_391_304u128), balance.amount.amount);
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
            let pending_reward_user_1_18s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 2608696 as reward is accrued
            assert_eq!(
                pending_reward_user_1_18s,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(2_608_696u128),
                    time_query: 1571797437
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
            assert_eq!(Uint128::from(32_608_695u128), balance.amount.amount);
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
                new_start_time: pool_info.phases_info[pool_info.current_phase_index as usize]
                    .end_time
                    + 10,
                new_end_time: pool_info.phases_info[pool_info.current_phase_index as usize]
                    .end_time
                    + 90,
                whitelist: Addr::unchecked(ADMIN.to_string()),
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
                msg: to_binary(&PoolQueryMsg::StakerInfo {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let staked_info_admin: StakerInfoResponse = from_binary(&res).unwrap();

            assert_eq!(
                staked_info_admin.amount,
                Uint128::from(ADD_1000_NATIVE_BALANCE_2)
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
            let pending_reward_admin_100s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 713_043_478 as reward is accrued
            assert_eq!(
                pending_reward_admin_100s,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(713_043_478u128),
                    time_query: 1571797519
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
            assert_eq!(Uint128::from(999_860_434_782u128), balance.amount.amount);
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
            let pending_reward_user_1_100s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 106_956_522 as reward is accrued
            assert_eq!(
                pending_reward_user_1_100s,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(106_956_522u128),
                    time_query: 1571797519
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
            assert_eq!(Uint128::from(32_608_695u128), balance.amount.amount);

            // Add 1000 NATIVE_DENOM_2 reward balance amount to pool contract by ADMIN
            let add_reward_balance_msg = PoolExecuteMsg::AddRewardBalance {
                phase_index: 1u64,
                amount: Uint128::from(ADD_1000_NATIVE_BALANCE_2),
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
            let pool_info_phase1: PoolInfos = app
                .wrap()
                .query_wasm_smart("contract3", &PoolQueryMsg::Pool {})
                .unwrap();

            // assert pool info
            assert_eq!(
                pool_info_phase1,
                PoolInfos {
                    staked_token: Addr::unchecked(lp_token_contract),
                    reward_token: native_token_info,
                    current_phase_index: 1u64,
                    phases_info: vec![
                        PhaseInfo {
                            start_time: pool_info.phases_info
                                [pool_info.current_phase_index as usize]
                                .start_time,
                            end_time: pool_info.phases_info[pool_info.current_phase_index as usize]
                                .end_time,
                            whitelist: Addr::unchecked(ADMIN.to_string()),
                            reward_balance: Uint128::from(ADD_1000_NATIVE_BALANCE_2),
                            last_reward_time: pool_info.phases_info
                                [pool_info.current_phase_index as usize]
                                .end_time,
                            accrued_token_per_share: Decimal::from_str("0.93043478260869565")
                                .unwrap(),
                            total_staked_at_end_time: Uint128::from(
                                MOCK_1000_HALO_LP_TOKEN_AMOUNT + MOCK_150_HALO_LP_TOKEN_AMOUNT
                            ),
                        },
                        PhaseInfo {
                            start_time: pool_info.phases_info
                                [pool_info.current_phase_index as usize]
                                .end_time
                                + 10,
                            end_time: pool_info.phases_info[pool_info.current_phase_index as usize]
                                .end_time
                                + 90,
                            whitelist: Addr::unchecked(ADMIN.to_string()),
                            reward_balance: Uint128::from(ADD_1000_NATIVE_BALANCE_2),
                            last_reward_time: pool_info.phases_info
                                [pool_info.current_phase_index as usize]
                                .end_time
                                + 10,
                            accrued_token_per_share: Decimal::zero(),
                            total_staked_at_end_time: Uint128::zero(),
                        }
                    ],
                    pool_limit_per_user: None,
                    staked_token_balance: Uint128::from(
                        MOCK_1000_HALO_LP_TOKEN_AMOUNT + MOCK_150_HALO_LP_TOKEN_AMOUNT
                    )
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
            let pending_reward_admin_135s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 217_391_304 as reward is accrued
            assert_eq!(
                pending_reward_admin_135s,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(217_391_304u128),
                    time_query: 1571797549
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
                msg: to_binary(&PoolQueryMsg::StakerInfo {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let staked_info_admin: StakerInfoResponse = from_binary(&res).unwrap();

            assert_eq!(
                staked_info_admin.amount,
                Uint128::from(ADD_1000_NATIVE_BALANCE_2)
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

            assert_eq!(balance.amount.amount, Uint128::from(999_077_826_086u128));

            // Query pending reward by USER_1 after 135 seconds
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: USER_1.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_user_1_135s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 106_956_522 + 32_608_695 = 139_565_217 as reward is accrued
            assert_eq!(
                pending_reward_user_1_135s,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(139_565_217u128),
                    time_query: 1571797549
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

            // It should be 32_608_695 + 106_956_522 + 32_608_695 = 172_173_912 as reward is accrued
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
            assert_eq!(balance.amount.amount, Uint128::from(172_173_912u128));

            // change block time increase 15 seconds to make 150 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(15),
                height: app.block_info().height + 15,
                chain_id: app.block_info().chain_id,
            });

            // Query pending reward by ADMIN after 150 seconds
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_admin_150s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 163_043_478 as reward is accrued
            assert_eq!(
                pending_reward_admin_150s,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(163_043_478u128),
                    time_query: 1571797564
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

            // It should be 999_240_869_564 as reward is accrued
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
                        + pending_reward_admin_150s.amount.u128()
                )
            );

            assert_eq!(balance.amount.amount, Uint128::from(999_240_869_564u128));

            // Query pending reward by USER_1 after 150 seconds
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: USER_1.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_user_1_150s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 24_456_522 as reward is accrued
            assert_eq!(
                pending_reward_user_1_150s,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(24_456_522u128),
                    time_query: 1571797564
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

            // It should be 32_608_695 + 106_956_522 + 32_608_695 + 24_456_522 = 196_630_434 as reward is accrued
            assert_eq!(
                balance.amount.amount,
                Uint128::from(
                    pending_reward_user1_2s.amount.u128()
                        + pending_reward_user1_8s.amount.u128()
                        + pending_reward_user_1_18s.amount.u128()
                        // + pending_reward_user_1_100s.amount.u128()
                        + pending_reward_user_1_135s.amount.u128() // Included pending_reward_user_1_100s
                        + pending_reward_user_1_150s.amount.u128()
                )
            );
            assert_eq!(balance.amount.amount, Uint128::from(196_630_434u128));

            // change block time increase 5 seconds to make 155 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(5),
                height: app.block_info().height + 5,
                chain_id: app.block_info().chain_id,
            });

            // Query pending reward by ADMIN after 155 seconds
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_admin_155s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 54_347_826 as reward is accrued
            assert_eq!(
                pending_reward_admin_155s,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(54_347_826u128),
                    time_query: 1571797569
                }
            );

            // Withdraw 50% ADMIN's staked LP amount from pool contract by ADMIN
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

            // Query staked info of ADMIN after withdraw
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::StakerInfo {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let staked_info_admin: StakerInfoResponse = from_binary(&res).unwrap();

            assert_eq!(
                staked_info_admin.amount,
                Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT / 2),
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
                    INIT_1000_000_NATIVE_BALANCE_2
                    - ADD_1000_NATIVE_BALANCE_2
                    - ADD_1000_NATIVE_BALANCE_2
                    + pending_reward_admin_2s.amount.u128()
                    + pending_reward_admin_6s.amount.u128()
                    + pending_reward_admin_10s.amount.u128()
                    + pending_reward_admin_14s.amount.u128()
                    // + pending_reward_admin_16s.amount.u128() // not execute harvest yet
                    + pending_reward_admin_18s.amount.u128() // Included pending_reward_admin_16s
                    + pending_reward_admin_100s.amount.u128()
                    + pending_reward_admin_135s.amount.u128()
                    + pending_reward_admin_150s.amount.u128()
                    + pending_reward_admin_155s.amount.u128()
                )
            );

            // Increase 5 second to make 160 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(5),
                height: app.block_info().height + 5,
                chain_id: app.block_info().chain_id,
            });

            // Query pending reward by USER_1 after 160 seconds
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: USER_1.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_user_1_160s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 22_575_251 as reward is accrued
            assert_eq!(
                pending_reward_user_1_160s,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(22_575_251u128),
                    time_query: 1571797574
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

            assert_eq!(
                balance.amount.amount,
                Uint128::from(
                    pending_reward_user1_2s.amount.u128()
                        + pending_reward_user1_8s.amount.u128()
                        + pending_reward_user_1_18s.amount.u128()
                        // + pending_reward_user_1_100s.amount.u128()
                        + pending_reward_user_1_135s.amount.u128() // Included pending_reward_user_1_100s
                        + pending_reward_user_1_150s.amount.u128()
                        + pending_reward_user_1_160s.amount.u128()
                )
            );

            // change block time increase 5 seconds to make 165 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(5),
                height: app.block_info().height + 5,
                chain_id: app.block_info().chain_id,
            });

            // Query pending reward by ADMIN after 165 seconds
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_admin_165s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 96_153_846 as reward is accrued
            assert_eq!(
                pending_reward_admin_165s,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(96_153_846u128),
                    time_query: 1571797579
                }
            );

            // Deposit 500 HALO LP token to the pool contract by ADMIN
            let deposit_msg = PoolExecuteMsg::Deposit {
                amount: Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT / 2),
            };

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

            // Execute deposit by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &deposit_msg,
                &[],
            );

            assert!(response.is_ok());

            // Change block time increase 5 seconds to make 170 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(5),
                height: app.block_info().height + 5,
                chain_id: app.block_info().chain_id,
            });

            // Query pending reward by ADMIN after 170 seconds
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_admin_170s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 54_347_826 as reward is accrued
            assert_eq!(
                pending_reward_admin_170s,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(54_347_826u128),
                    time_query: 1571797584
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
                    INIT_1000_000_NATIVE_BALANCE_2
                    - ADD_1000_NATIVE_BALANCE_2
                    - ADD_1000_NATIVE_BALANCE_2
                    + pending_reward_admin_2s.amount.u128()
                    + pending_reward_admin_6s.amount.u128()
                    + pending_reward_admin_10s.amount.u128()
                    + pending_reward_admin_14s.amount.u128()
                    // + pending_reward_admin_16s.amount.u128() // not execute harvest yet
                    + pending_reward_admin_18s.amount.u128() // Included pending_reward_admin_16s
                    + pending_reward_admin_100s.amount.u128()
                    + pending_reward_admin_135s.amount.u128()
                    + pending_reward_admin_150s.amount.u128()
                    + pending_reward_admin_155s.amount.u128()
                    + pending_reward_admin_165s.amount.u128()
                    + pending_reward_admin_170s.amount.u128()
                )
            );
        }

        // Phase 0:
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
        //
        // Increase 92 seconds to make 100 seconds passed
        // -> 92s: HALO REWARD token for ADMIN: 6,666 * 92 = 613,33 (Not harvest yet)
        //       : HALO REWARD token for USER_1:  3,334 * 92 = 306,666 (Not harvest yet)
        //
        // Phase 1:
        //
        // Extend end time to 10 more seconds by ADMIN
        // Mint 1000 HALO REWARD token for ADMIN
        // Add 1000 HALO REWARD token reward balance to pool contract by ADMIN
        // -> 100 HALO REWARD token per second
        // Increase 1 second to make 101 seconds passed
        // -> 1s: HALO REWARD token for ADMIN: 613,33 + 66,66 = 679,99 (Not harvest yet)
        //      : HALO REWARD token for USER_1: 306,666 + 33,334 = 340 (Not harvest yet)
        // ADMIN Withdraw 500 HALO LP token to the pool contract
        // ADMIN Send 500 HALO LP token to USER_1
        // USER_1 Deposit 500 HALO LP token to the pool contract
        //
        // Phase 2:
        //
        // Extend a new phase with 10 more seconds by ADMIN
        // Add 10 HALO REWARD token reward balance to pool contract by ADMIN
        // Remove this new phase by ADMIN
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
                contract_addr: Addr::unchecked(reward_token_contract.clone()),
            };

            // create pool contract by factory contract
            let create_pool_msg = crate::msg::ExecuteMsg::CreatePool {
                staked_token: Addr::unchecked(lp_token_contract.clone()),
                reward_token: reward_token_info.clone(),
                start_time: current_block_time,
                end_time: current_block_time + 100,
                pool_limit_per_user: None,
                whitelist: Addr::unchecked(ADMIN.to_string()),
            };

            // Execute create pool by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(factory_contract.clone()),
                &create_pool_msg,
                &[],
            );

            assert!(response.is_ok());

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
                    staked_token: Addr::unchecked(lp_token_contract),
                    reward_token: reward_token_info.clone(),
                    start_time: current_block_time,
                    end_time: current_block_time + 100,
                    pool_limit_per_user: None,
                }
            );

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
                amount: Uint128::from(MOCK_1000_HALO_REWARD_TOKEN_AMOUNT),
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
            let pending_reward_admin_2s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 20x10^18 as reward is accrued
            assert_eq!(
                pending_reward_admin_2s,
                RewardTokenAssetResponse {
                    info: TokenInfo::Token {
                        contract_addr: Addr::unchecked(reward_token_contract.clone()),
                    },
                    amount: Uint128::from(20_000_000_000_000_000_000u128),
                    time_query: 1571797421,
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
            let pending_reward_user1_4s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 6,6666x10^18 as reward is accrued
            assert_eq!(
                pending_reward_user1_4s,
                RewardTokenAssetResponse {
                    info: TokenInfo::Token {
                        contract_addr: Addr::unchecked(reward_token_contract.clone()),
                    },
                    amount: Uint128::from(6_666_666_666_666_666_666u128),
                    time_query: 1571797423,
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
            let pending_reward_admin_6s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 26,666x10^18 as reward is accrued
            assert_eq!(
                pending_reward_admin_6s,
                RewardTokenAssetResponse {
                    info: TokenInfo::Token {
                        contract_addr: Addr::unchecked(reward_token_contract.clone()),
                    },
                    amount: Uint128::from(26_666_666_666_666_666_666u128),
                    time_query: 1571797425,
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
            let pending_reward_admin_7s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 5x10^18 as reward is accrued
            assert_eq!(
                pending_reward_admin_7s,
                RewardTokenAssetResponse {
                    info: TokenInfo::Token {
                        contract_addr: Addr::unchecked(reward_token_contract.clone()),
                    },
                    amount: Uint128::from(5_000_000_000_000_000_000u128),
                    time_query: 1571797426,
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
            let pending_reward_admin_8s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 6,66x10^18 as reward is accrued
            assert_eq!(
                pending_reward_admin_8s,
                RewardTokenAssetResponse {
                    info: TokenInfo::Token {
                        contract_addr: Addr::unchecked(reward_token_contract.clone())
                    },
                    amount: Uint128::from(6_666_666_666_666_666_667u128),
                    time_query: 1571797427,
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

            // query pending reward by USER_1 after 8 seconds
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: USER_1.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_user1_8s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 15x10^18 as reward is accrued
            assert_eq!(
                pending_reward_user1_8s,
                RewardTokenAssetResponse {
                    info: TokenInfo::Token {
                        contract_addr: Addr::unchecked(reward_token_contract.clone())
                    },
                    amount: Uint128::from(15_000_000_000_000_000_000u128),
                    time_query: 1571797427,
                }
            );

            // harvest reward by USER_1
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
            assert_eq!(
                balance.balance,
                pending_reward_user1_4s.amount + pending_reward_user1_8s.amount
            );

            // Query total LP staked by calling TotalStaked query
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::TotalStaked {}).unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let total_staked: Uint128 = from_binary(&res).unwrap();

            // It should be 1000 HALO LP token
            assert_eq!(
                total_staked,
                Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT + MOCK_1000_HALO_LP_TOKEN_AMOUNT / 2)
            );

            // Extend end time by ADMIN more 10 seconds
            let extend_end_time_msg = PoolExecuteMsg::AddPhase {
                new_start_time: pool_info.end_time,
                new_end_time: pool_info.end_time + 10,
                whitelist: Addr::unchecked(ADMIN.to_string()),
            };

            // Execute extend end time by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &extend_end_time_msg,
                &[],
            );

            assert!(response.is_ok());

            // change block time increase 92 seconds to make 100 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(92),
                height: app.block_info().height + 92,
                chain_id: app.block_info().chain_id,
            });

            // query pending reward by ADMIN after 100 seconds (Not harvest yet)
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_admin_100s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 920x10^18 as reward is accrued
            assert_eq!(
                pending_reward_admin_100s,
                RewardTokenAssetResponse {
                    info: TokenInfo::Token {
                        contract_addr: Addr::unchecked(reward_token_contract.clone())
                    },
                    amount: Uint128::from(613_333_333_333_333_333_333u128),
                    time_query: 1571797519,
                }
            );

            // query pending reward by USER_1 after 100 seconds (Not harvest yet)
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: USER_1.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_user_1_100s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 306,666x10^18 as reward is accrued

            assert_eq!(
                pending_reward_user_1_100s,
                RewardTokenAssetResponse {
                    info: TokenInfo::Token {
                        contract_addr: Addr::unchecked(reward_token_contract.clone())
                    },
                    amount: Uint128::from(306_666_666_666_666_666_667u128),
                    time_query: 1571797519,
                }
            );

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
                phase_index: 1u64,
                amount: Uint128::from(MOCK_1000_HALO_REWARD_TOKEN_AMOUNT),
            };

            // Execute add reward by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &add_reward_balance_msg,
                &[],
            );

            assert!(response.is_ok());

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

            // change block time increase 1 seconds to make 101 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(1),
                height: app.block_info().height + 1,
                chain_id: app.block_info().chain_id,
            });

            // query pending reward by ADMIN after 101 seconds (Not harvest yet)
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_admin_101s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 679,999^18 as reward is accrued
            assert_eq!(
                pending_reward_admin_101s,
                RewardTokenAssetResponse {
                    info: TokenInfo::Token {
                        contract_addr: Addr::unchecked(reward_token_contract.clone())
                    },
                    amount: Uint128::from(679_999_999_999_999_999_999u128),
                    time_query: 1571797520,
                }
            );

            // query pending reward by USER_1 after 101 seconds (Not harvest yet)
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: USER_1.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_user_1_101s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 340,000^18 as reward is accrued
            assert_eq!(
                pending_reward_user_1_101s,
                RewardTokenAssetResponse {
                    info: TokenInfo::Token {
                        contract_addr: Addr::unchecked(reward_token_contract.clone())
                    },
                    amount: Uint128::from(340_000_000_000_000_000_000u128),
                    time_query: 1571797520,
                }
            );

            // ADMIN Withdraw 500 HALO LP token to the pool contract
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

            // It should be 679,999^18 reward token
            assert_eq!(
                balance.balance,
                pending_reward_admin_2s.amount
                    + pending_reward_admin_6s.amount
                    + pending_reward_admin_7s.amount
                    + pending_reward_admin_8s.amount
                    // + pending_reward_admin_100s.amount
                    + pending_reward_admin_101s.amount // Included 100s reward token
            );

            // ADMIN Send 500 HALO LP token to USER_1
            let transfer_msg: Cw20ExecuteMsg = Cw20ExecuteMsg::Transfer {
                recipient: USER_1.to_string(),
                amount: Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT / 2),
            };

            // Execute transfer by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(lp_token_contract.clone()),
                &transfer_msg,
                &[],
            );

            assert!(response.is_ok());

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

            assert_eq!(
                balance.balance,
                pending_reward_user1_4s.amount
                    + pending_reward_user1_8s.amount
                    // + pending_reward_user_1_100s.amount
                    + pending_reward_user_1_101s.amount // Included 100s reward token
            );

            // Extend end time by ADMIN more 10 seconds
            let extend_end_time_msg = PoolExecuteMsg::AddPhase {
                new_start_time: pool_info.end_time + 10,
                new_end_time: pool_info.end_time + 20,
                whitelist: Addr::unchecked(ADMIN.to_string()),
            };

            // Execute extend end time by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &extend_end_time_msg,
                &[],
            );

            assert!(response.is_ok());

            // Increase allowance of reward token to pool contract
            let approve_msg: Cw20ExecuteMsg = Cw20ExecuteMsg::IncreaseAllowance {
                spender: "contract3".to_string(), // Pool Contract
                amount: Uint128::from(10_000_000_000_000_000_000u128),
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

            // Add 10 HALO reward tokens to pool contract
            let add_reward_balance_msg = PoolExecuteMsg::AddRewardBalance {
                phase_index: 2u64,
                amount: Uint128::from(10_000_000_000_000_000_000u128),
            };

            // Execute add reward by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &add_reward_balance_msg,
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

            // It should be 679,999^18 reward token
            assert_eq!(
                balance.balance,
                pending_reward_admin_2s.amount
                    + pending_reward_admin_6s.amount
                    + pending_reward_admin_7s.amount
                    + pending_reward_admin_8s.amount
                    // + pending_reward_admin_100s.amount
                    + pending_reward_admin_101s.amount // Included 100s reward token
                    - Uint128::from(10_000_000_000_000_000_000u128) // 10 HALO reward token
            );

            // Remove phase 2
            let remove_phase_msg = PoolExecuteMsg::RemovePhase { phase_index: 2u64 };

            // Execute remove phase by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &remove_phase_msg,
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

            // It should be 679,999^18 reward token
            assert_eq!(
                balance.balance,
                pending_reward_admin_2s.amount
                    + pending_reward_admin_6s.amount
                    + pending_reward_admin_7s.amount
                    + pending_reward_admin_8s.amount
                    // + pending_reward_admin_100s.amount
                    + pending_reward_admin_101s.amount // Included 100s reward token
                    - Uint128::from(10_000_000_000_000_000_000u128) // 10 HALO reward token
                    + Uint128::from(10_000_000_000_000_000_000u128) // 10 HALO reward token back
            );
        }

        // Create pool contract with 2 phases
        // ----- Phase 0 -----
        // Add 1000 NATIVE_2 reward balance amount to pool contract by ADMIN in phase 0
        // with end time 10 seconds -> 100 NATIVE_2 per second
        // Increase 2 seconds
        // Deposit 1000 lp token to the pool contract by ADMIN
        // Increase 8 seconds
        // -> ADMIN Reward balance 8s: 800 NATIVE_2 (Not claim yet)
        // ----- Phase 1 -----
        // Extend end time by ADMIN more 10 seconds with start_time = Phase 0's end_time + 2 seconds
        // Increase 1 second
        // Add 1000 NATIVE_2 reward balance amount to pool contract by ADMIN in phase 1
        // with end time 10 seconds -> 100 NATIVE_2 per second
        // Increase 1 second to make 12 seconds passed -> Phase 1 starts
        // Increase 2 seconds to make 14 seconds passed
        // Deposit 500 lp token to the pool contract by USER_1
        // -> ADMIN Reward balance 14s: 200 NATIVE_2 (Not claim yet)
        // Increase 6 second
        // USER_1 Pending reward 6s: (500 / (1000 + 500) * 6 * 100) = 200 NATIVE_2
        // Harvest reward by ADMIN after 6 seconds by depositing more 1000 lp token
        // -> Reward balance 20s: 800 + 200 + (1000 / (1000 + 500) * 6 * 100) = 1400 NATIVE_2
        // Increase 2 second
        // USER_1 Harvest reward after 8 seconds
        // USER_1 Pending reward 6s: 200 NATIVE_2
        // ->                    2s: (500 / (1000 + 500 + 1000) * 2 * 100) = 40 NATIVE_2
        // ADMIN pending reward 2s: (2000 / (1000 + 500 + 1000) * 2 * 100) = 160 NATIVE_2
        // Increase 2 second to make 24 seconds passed out of 2 seconds Phases 1's passed
        //
        // USER_1 Harvest reward
        // -> Reward balance 24s == Reward balance 20s = 240 NATIVE_2
        // Increase 1s (25 seconds passed)
        // ADMIN pending reward 2s: (2000 / (1000 + 500 + 1000) * 2 * 100) = 160 NATIVE_2 (Re-check)
        // ----- Phase 2 -----
        // Increase 1s (26 seconds passed)
        // Extend end time by ADMIN more 10 seconds with start_time at 29 seconds
        // Increase 1s (27 seconds passed)
        // Add 1000 NATIVE_2 reward balance amount to pool contract by ADMIN in phase 2
        // Increase 1s (28 seconds passed)
        // Activate phase 2
        // Increase 1s (29 seconds passed)
        // Query pending reward by ADMIN after 29 seconds (Not harvest yet)
        // -> Reward balance 29s: 160 NATIVE_2
        // Increase 1s (30 seconds passed)
        // Query pending reward by ADMIN after 30 seconds (Not harvest yet)
        // -> Reward balance 29s: 160 NATIVE_2
        //                    1s: (2000 / (1000 + 500 + 1000) * 1 * 100) = 80 NATIVE_2
        // -> Reward balance 30s: 240 NATIVE_2
        #[test]
        fn proper_harvest_with_multipe_phases() {
            // get integration test app and contracts
            let (mut app, contracts) = instantiate_contracts();
            // ADMIN already has 1_000_000 NATIVE_DENOM_2 as initial balance in instantiate_contracts()
            // get pool factory contract
            let factory_contract = &contracts[0].contract_addr;
            // get halo lp token contract
            let lp_token_contract = &contracts[1].contract_addr;
            // get current block time
            let current_block_time = app.block_info().time.seconds();

            // Mint 10_000 HALO LP tokens to ADMIN
            let mint_msg: Cw20ExecuteMsg = Cw20ExecuteMsg::Mint {
                recipient: ADMIN.to_string(),
                amount: Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT * 10),
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
                staked_token: Addr::unchecked(lp_token_contract.clone()),
                reward_token: native_token_info.clone(),
                start_time: current_block_time,
                end_time: current_block_time + 10,
                pool_limit_per_user: None,
                whitelist: Addr::unchecked(ADMIN.to_string()),
            };

            // Execute create pool
            let response_create_pool = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(factory_contract.clone()),
                &create_pool_msg,
                &[],
            );

            assert!(response_create_pool.is_ok());

            // add reward balance to pool contract
            let add_reward_balance_msg = PoolExecuteMsg::AddRewardBalance {
                phase_index: 0u64,
                amount: Uint128::from(ADD_1000_NATIVE_BALANCE_2),
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
            let pool_info: PoolInfos = app
                .wrap()
                .query_wasm_smart("contract3", &PoolQueryMsg::Pool {})
                .unwrap();

            // assert pool info
            assert_eq!(
                pool_info,
                PoolInfos {
                    staked_token: Addr::unchecked(lp_token_contract),
                    reward_token: native_token_info,
                    current_phase_index: 0u64,
                    phases_info: vec![PhaseInfo {
                        start_time: current_block_time,
                        end_time: current_block_time + 10,
                        whitelist: Addr::unchecked(ADMIN.to_string()),
                        reward_balance: Uint128::from(ADD_1000_NATIVE_BALANCE_2),
                        last_reward_time: current_block_time,
                        accrued_token_per_share: Decimal::zero(),
                        total_staked_at_end_time: Uint128::zero(),
                    }],
                    pool_limit_per_user: None,
                    staked_token_balance: Uint128::zero(),
                }
            );

            // change block time increase 2 seconds to make 2 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(2),
                height: app.block_info().height + 2,
                chain_id: app.block_info().chain_id,
            });

            // Approve cw20 token to pool contract msg
            let approve_msg: Cw20ExecuteMsg = Cw20ExecuteMsg::IncreaseAllowance {
                spender: "contract3".to_string(), // Pool Contract
                amount: Uint128::from(MOCK_1000_HALO_LP_TOKEN_AMOUNT * 10),
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

            // Deposit lp token to the pool contract by ADMIN
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

            // change block time increase 8 seconds to make 10 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(8),
                height: app.block_info().height + 8,
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
            let pending_reward_admin_8s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 800 NATIVE_2 as reward is accrued
            assert_eq!(
                pending_reward_admin_8s,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(800_000_000u128),
                    time_query: 1571797429,
                }
            );

            // ----- Phase 1 -----
            // Extend end time by ADMIN more 10 seconds
            let extend_end_time_msg = PoolExecuteMsg::AddPhase {
                new_start_time: pool_info.phases_info[pool_info.current_phase_index as usize]
                    .end_time
                    + 2,
                new_end_time: pool_info.phases_info[pool_info.current_phase_index as usize]
                    .end_time
                    + 12,
                whitelist: Addr::unchecked(ADMIN.to_string()),
            };

            // Execute extend end time by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &extend_end_time_msg,
                &[],
            );

            assert!(response.is_ok());

            // add reward balance to pool contract
            let add_reward_balance_msg = PoolExecuteMsg::AddRewardBalance {
                phase_index: 1u64,
                amount: Uint128::from(ADD_1000_NATIVE_BALANCE_2),
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

            // increase block time 1 seconds to make 11 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(1),
                height: app.block_info().height + 1,
                chain_id: app.block_info().chain_id,
            });

            // activate phase 1
            let activate_phase_msg = PoolExecuteMsg::ActivatePhase {};

            // Execute activate phase by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &activate_phase_msg,
                &[],
            );

            assert!(response.is_ok());

            // increase block time 1 seconds to make 12 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(1),
                height: app.block_info().height + 1,
                chain_id: app.block_info().chain_id,
            });

            // PHASE 1 STARTS

            // change block time increase 2 seconds to make 14 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(2),
                height: app.block_info().height + 2,
                chain_id: app.block_info().chain_id,
            });

            // query pending reward by ADMIN after 14 seconds
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_admin_14s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 1000 NATIVE_2 as reward is accrued
            assert_eq!(
                pending_reward_admin_14s,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(1_000_000_000u128),
                    time_query: 1571797433,
                }
            );

            // Approve cw20 token to pool contract msg
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

            // Deposit lp token to the pool contract
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

            // change block time increase 6 seconds to make 20 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(6),
                height: app.block_info().height + 6,
                chain_id: app.block_info().chain_id,
            });

            // query pending reward by ADMIN after 20 seconds
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_admin_20s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 1400 NATIVE_2 as reward is accrued
            assert_eq!(
                pending_reward_admin_20s,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(1_400_000_000u128),
                    time_query: 1571797439,
                }
            );

            // query pending reward by USER_1 after 6 seconds
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: USER_1.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_user1_6s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 200 NATIVE_2 as reward is accrued
            assert_eq!(
                pending_reward_user1_6s,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(200_000_000u128),
                    time_query: 1571797439,
                }
            );

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
                Uint128::from(
                    NATIVE_BALANCE_2 - ADD_1000_NATIVE_BALANCE_2 - ADD_1000_NATIVE_BALANCE_2
                )
            );

            // deposit 1000 lp token to the pool contract by ADMIN
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
                Uint128::from(
                    NATIVE_BALANCE_2 - ADD_1000_NATIVE_BALANCE_2 - ADD_1000_NATIVE_BALANCE_2
                        + pending_reward_admin_20s.amount.u128()
                )
            );

            // query pending reward of ADMIN after harvest
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_admin_harvest: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 0 NATIVE_2 as reward is accrued
            assert_eq!(
                pending_reward_admin_harvest,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::zero(),
                    time_query: 1571797439,
                }
            );

            // query pending reward by USER_1 after 6 seconds after ADMIN harvest
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: USER_1.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_user1_6s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 200 NATIVE_2 as reward is accrued
            assert_eq!(
                pending_reward_user1_6s,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(200_000_000u128),
                    time_query: 1571797439,
                }
            );

            // change block time increase 2 seconds to make 22 seconds passed -- END OF PHASE 1 --
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(2),
                height: app.block_info().height + 2,
                chain_id: app.block_info().chain_id,
            });

            // query pending reward by USER_1 after 8 seconds after ADMIN harvest
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: USER_1.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_user1_8s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 200 NATIVE_2 as reward is accrued
            assert_eq!(
                pending_reward_user1_8s,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(240_000_000u128),
                    time_query: 1571797441,
                }
            );

            // query pending reward by ADMIN
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_admin_8s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 800 NATIVE_2 as reward is accrued
            assert_eq!(
                pending_reward_admin_8s,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(160_000_000u128),
                    time_query: 1571797441,
                }
            );

            // Increase 2 second to make 24 seconds passed out of 2 seconds Phases 1's passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(2),
                height: app.block_info().height + 2,
                chain_id: app.block_info().chain_id,
            });

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

            // It should be 240 NATIVE_2 as reward is accrued
            assert_eq!(
                balance.amount.amount,
                Uint128::from(pending_reward_user1_8s.amount.u128())
            );

            // Increase 1 second to make 25 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(1),
                height: app.block_info().height + 1,
                chain_id: app.block_info().chain_id,
            });

            // query pending reward by ADMIN after 25 seconds after USER_1 harvest(Re-check)
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_admin_25s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 160 NATIVE_2 as reward is accrued
            assert_eq!(
                pending_reward_admin_25s,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(160_000_000u128),
                    time_query: 1571797444,
                }
            );

            // Increase 1 second to make 26 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(1),
                height: app.block_info().height + 1,
                chain_id: app.block_info().chain_id,
            });

            // ----- Phase 2 -----
            // Extend end time by ADMIN more 10 seconds with start_time at 29 seconds
            let extend_end_time_msg = PoolExecuteMsg::AddPhase {
                new_start_time: 1571797448, // 29 seconds
                new_end_time: 1571797448 + 10,
                whitelist: Addr::unchecked(ADMIN.to_string()),
            };

            // Execute extend end time by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &extend_end_time_msg,
                &[],
            );

            assert!(response.is_ok());

            // Increase 1 second to make 27 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(1),
                height: app.block_info().height + 1,
                chain_id: app.block_info().chain_id,
            });

            // add reward balance to pool contract
            let add_reward_balance_msg = PoolExecuteMsg::AddRewardBalance {
                phase_index: 2u64,
                amount: Uint128::from(ADD_1000_NATIVE_BALANCE_2),
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

            // query balance of ADMIN in native token
            let req: QueryRequest<BankQuery> = QueryRequest::Bank(BankQuery::Balance {
                address: ADMIN.to_string(),
                denom: NATIVE_DENOM_2.to_string(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let balance: BankBalanceResponse = from_binary(&res).unwrap();

            assert_eq!(balance.amount.amount, Uint128::from(998_400_000_000u128));

            // Increase 1 second to make 28 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(1),
                height: app.block_info().height + 1,
                chain_id: app.block_info().chain_id,
            });

            // Activate phase 2
            let activate_phase_msg = PoolExecuteMsg::ActivatePhase {};

            // Execute activate phase by ADMIN
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &activate_phase_msg,
                &[],
            );

            assert!(response.is_ok());

            // Increase 1 second to make 29 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(1),
                height: app.block_info().height + 1,
                chain_id: app.block_info().chain_id,
            });

            // query pending reward by ADMIN after 29 seconds after USER_1 harvest(Re-check)
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_admin_29s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 160 NATIVE_2 as reward is accrued
            assert_eq!(
                pending_reward_admin_29s,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(160_000_000u128),
                    time_query: 1571797448,
                }
            );

            // Increase 1 second to make 30 seconds passed
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(1),
                height: app.block_info().height + 1,
                chain_id: app.block_info().chain_id,
            });

            // query pending reward by ADMIN after 30 seconds after USER_1 harvest
            let req: QueryRequest<PoolQueryMsg> = QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "contract3".to_string(),
                msg: to_binary(&PoolQueryMsg::PendingReward {
                    address: ADMIN.to_string(),
                })
                .unwrap(),
            });

            let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
            let pending_reward_admin_30s: RewardTokenAssetResponse = from_binary(&res).unwrap();

            // It should be 240 NATIVE_2 as reward is accrued
            assert_eq!(
                pending_reward_admin_30s,
                RewardTokenAssetResponse {
                    info: TokenInfo::NativeToken {
                        denom: NATIVE_DENOM_2.to_string()
                    },
                    amount: Uint128::from(240_000_000u128),
                    time_query: 1571797449,
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
                Uint128::from(998_400_000_000u128 + pending_reward_admin_30s.amount.u128())
            );
        }
    }
}
