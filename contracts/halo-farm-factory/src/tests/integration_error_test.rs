#![cfg(test)]
mod tests {
    const _MOCK_1000_000_000_HALO_LP_TOKEN_AMOUNT: u128 = 1_000_000_000_000_000;
    const MOCK_1000_HALO_LP_TOKEN_AMOUNT: u128 = 1_000_000_000;
    const MOCK_1000_HALO_REWARD_TOKEN_AMOUNT: u128 = 1_000_000_000_000_000_000_000;
    const _MOCK_150_000_000_HALO_LP_TOKEN_AMOUNT: u128 = 150_000_000_000_000;
    const MOCK_150_HALO_LP_TOKEN_AMOUNT: u128 = 150_000_000;
    const INIT_1000_000_NATIVE_BALANCE_2: u128 = 1_000_000_000_000u128;
    const ADD_1000_NATIVE_BALANCE_2: u128 = 1_000_000_000u128;
    mod execute_error_operation {
        use std::str::FromStr;

        use cosmwasm_std::{
            from_binary, to_binary, Addr, BalanceResponse as BankBalanceResponse, BankQuery,
            BlockInfo, Coin, Decimal, Querier, QueryRequest, Uint128, WasmQuery,
        };
        use cw20::{BalanceResponse, Cw20ExecuteMsg};
        use cw_multi_test::Executor;
        use halo_farm::state::{
            PhaseInfo, PoolInfos, RewardTokenAsset, RewardTokenAssetResponse, StakerInfoResponse,
            TokenInfo,
        };

        use crate::{
            msg::QueryMsg,
            state::FactoryPoolInfo,
            tests::{
                env_setup::env::{
                    instantiate_contracts, ADMIN, NATIVE_BALANCE_2, NATIVE_DENOM_2, USER_1,
                },
                integration_error_test::tests::{
                    ADD_1000_NATIVE_BALANCE_2, INIT_1000_000_NATIVE_BALANCE_2,
                    MOCK_1000_HALO_LP_TOKEN_AMOUNT, MOCK_1000_HALO_REWARD_TOKEN_AMOUNT,
                    MOCK_150_HALO_LP_TOKEN_AMOUNT,
                },
            },
        };
        use halo_farm::msg::{ExecuteMsg as PoolExecuteMsg, QueryMsg as PoolQueryMsg};

        // Create pool
        // 1. Unauthorized create pool
        // 2. Fail to crease pool with Start time > end time
        // 3. Fail to crease pool with Current time > start time
        // 4. Fail to Remove 0 phase
        // 5. Unauthorized remove phase
        // 6. Fail to activate empty phase
        // 7. Unauthorized activate phase
        #[test]
        fn farm_factory_create_farm() {
            // get integration test app and contracts
            let (mut app, contracts) = instantiate_contracts();
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

            // native token info
            let native_token_info = TokenInfo::NativeToken {
                denom: NATIVE_DENOM_2.to_string(),
            };

            // create pool contract by factory contract with unauthorized user
            let create_pool_msg = crate::msg::ExecuteMsg::CreatePool {
                staked_token: Addr::unchecked(lp_token_contract.clone()),
                reward_token: native_token_info.clone(),
                start_time: current_block_time,
                end_time: current_block_time + 100,
                pool_limit_per_user: None,
                whitelist: Addr::unchecked(USER_1.to_string()),
            };

            // Execute create pool with unauthorized user
            let response_create_pool = app.execute_contract(
                Addr::unchecked(USER_1.to_string()), // unauthorized user
                Addr::unchecked(factory_contract.clone()),
                &create_pool_msg,
                &[],
            );

            // check unauthorized error
            assert_eq!(
                response_create_pool
                    .unwrap_err()
                    .source()
                    .unwrap()
                    .to_string(),
                "Unauthorized"
            );

            // create pool contract by factory contract with start time > end time
            let create_pool_msg = crate::msg::ExecuteMsg::CreatePool {
                staked_token: Addr::unchecked(lp_token_contract.clone()),
                reward_token: native_token_info.clone(),
                start_time: current_block_time + 100, // start time > end time
                end_time: current_block_time,
                pool_limit_per_user: None,
                whitelist: Addr::unchecked(USER_1.to_string()),
            };

            // Execute create pool with start time > end time
            let response_create_pool = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(factory_contract.clone()),
                &create_pool_msg,
                &[],
            );

            // check start time > end time error
            assert_eq!(
                response_create_pool
                    .unwrap_err()
                    .source()
                    .unwrap()
                    .to_string(),
                "Generic error: Start time is greater than end time"
            );

            // create pool contract by factory contract with current time > start time
            let create_pool_msg = crate::msg::ExecuteMsg::CreatePool {
                staked_token: Addr::unchecked(lp_token_contract.clone()),
                reward_token: native_token_info.clone(),
                start_time: current_block_time - 1, // current time > start time
                end_time: current_block_time + 200,
                pool_limit_per_user: None,
                whitelist: Addr::unchecked(USER_1.to_string()),
            };

            // Execute create pool with current time > start time
            let response_create_pool = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(factory_contract.clone()),
                &create_pool_msg,
                &[],
            );

            // check current time > start time error
            assert_eq!(
                response_create_pool
                    .unwrap_err()
                    .source()
                    .unwrap()
                    .to_string(),
                "Generic error: Current time is greater than start time"
            );

            // remove phase 0
            // successfully create pool contract by factory contract
            let create_pool_msg = crate::msg::ExecuteMsg::CreatePool {
                staked_token: Addr::unchecked(lp_token_contract.clone()),
                reward_token: native_token_info.clone(),
                start_time: current_block_time,
                end_time: current_block_time + 200,
                pool_limit_per_user: None,
                whitelist: Addr::unchecked(USER_1.to_string()),
            };

            // Execute create pool
            let response_create_pool = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(factory_contract.clone()),
                &create_pool_msg,
                &[],
            );

            // check create pool success
            assert!(response_create_pool.is_ok());

            let remove_phase_msg = halo_farm::msg::ExecuteMsg::RemovePhase { phase_index: 0 };

            // Execute remove phase 0
            let response_remove_phase = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3".clone()),
                &remove_phase_msg,
                &[],
            );

            // check remove phase 0 error
            assert_eq!(
                response_remove_phase
                    .unwrap_err()
                    .source()
                    .unwrap()
                    .to_string(),
                "Generic error: Can not remove activated phase"
            );

            // successfully add phase 1
            let add_phase_msg = halo_farm::msg::ExecuteMsg::AddPhase {
                new_start_time: current_block_time + 200,
                new_end_time: current_block_time + 400,
                whitelist: Addr::unchecked(USER_1.to_string()),
            };

            // Execute add phase 1
            let response_add_phase = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3".clone()),
                &add_phase_msg,
                &[],
            );

            // check add phase 1 success
            assert!(response_add_phase.is_ok());

            // USER_1 remove phase 1
            let remove_phase_msg = halo_farm::msg::ExecuteMsg::RemovePhase { phase_index: 1 };

            // Execute remove phase 1
            let response_remove_phase = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3".clone()),
                &remove_phase_msg,
                &[],
            );

            // check remove phase 1 error
            assert_eq!(
                response_remove_phase
                    .unwrap_err()
                    .source()
                    .unwrap()
                    .to_string(),
                "Generic error: Unauthorized: Only owner can remove phase"
            );

            // fail to activate empty phase
            // change block time increase 200 seconds to phase 1's start time
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(200),
                height: app.block_info().height + 1,
                chain_id: app.block_info().chain_id,
            });

            let activate_phase_msg = halo_farm::msg::ExecuteMsg::ActivatePhase {};

            // Execute activate phase 1
            let response_activate_phase = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3".clone()),
                &activate_phase_msg,
                &[],
            );

            // check activate phase 1 error
            assert_eq!(
                response_activate_phase
                    .unwrap_err()
                    .source()
                    .unwrap()
                    .to_string(),
                "Generic error: Empty phase"
            );

            // USER_1 activate phase 1
            let activate_phase_msg = halo_farm::msg::ExecuteMsg::ActivatePhase {};

            // Execute activate phase 1
            let response_activate_phase = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3".clone()),
                &activate_phase_msg,
                &[],
            );

            // check activate phase 1 error
            assert_eq!(
                response_activate_phase
                    .unwrap_err()
                    .source()
                    .unwrap()
                    .to_string(),
                "Generic error: Unauthorized: Only owner can active new phase"
            );
        }
    }
}
