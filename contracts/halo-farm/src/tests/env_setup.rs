#[cfg(test)]
pub mod env {
    use cosmwasm_std::{Addr, Coin, Empty, Uint128};
    use cw20::MinterResponse;
    use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};

    use cw20_base::contract::{
        execute as Cw20Execute, instantiate as Cw20Instantiate, query as Cw20Query,
    };

    use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;

    use crate::contract::{
        execute as HaloFarmExecute, instantiate as HaloFarmInstantiate, query as HaloFarmQuery,
    };

    pub const ADMIN: &str = "aura1000000000000000000000000000000000admin";
    pub const USER_1: &str = "aura1000000000000000000000000000000000user1";

    pub const NATIVE_DENOM: &str = "uaura";
    pub const NATIVE_BALANCE: u128 = 1_000_000_000_000u128;

    pub const NATIVE_DENOM_2: &str = "utaura";
    pub const NATIVE_BALANCE_2: u128 = 1_000_000_000_000u128;

    pub struct ContractInfo {
        pub contract_addr: String,
        pub contract_code_id: u64,
    }

    fn mock_app() -> App {
        AppBuilder::new().build(|router, _, storage| {
            router
                .bank
                .init_balance(
                    storage,
                    &Addr::unchecked(ADMIN),
                    vec![
                        Coin {
                            denom: NATIVE_DENOM.to_string(),
                            amount: Uint128::new(NATIVE_BALANCE),
                        },
                        Coin {
                            denom: NATIVE_DENOM_2.to_string(),
                            amount: Uint128::new(NATIVE_BALANCE_2),
                        },
                    ],
                )
                .unwrap();
        })
    }

    pub fn halo_farm_contract_template() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(HaloFarmExecute, HaloFarmInstantiate, HaloFarmQuery);
        Box::new(contract)
    }

    // halo lp token contract
    // create instantiate message for contract
    fn halo_lp_token_contract_template() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(Cw20Execute, Cw20Instantiate, Cw20Query);
        Box::new(contract)
    }

    pub fn instantiate_contracts() -> (App, Vec<ContractInfo>) {
        // Create a new app instance
        let mut app = mock_app();
        // Create a vector to store all contract info ([halo factory - [0])
        let mut contract_info_vec: Vec<ContractInfo> = Vec::new();

        // store code of all contracts to the app and get the code ids
        let halo_lp_token_contract_code_id = app.store_code(halo_lp_token_contract_template());
        let halo_reward_decimal_18_token_contract_code_id =
            app.store_code(halo_lp_token_contract_template());

        // halo lp token contract
        // create instantiate message for contract
        let halo_lp_token_instantiate_msg = Cw20InstantiateMsg {
            name: "Halo LP Token".to_string(),
            symbol: "HALO-LP".to_string(),
            decimals: 6,
            initial_balances: vec![],
            mint: Some(MinterResponse {
                minter: ADMIN.to_string(),
                cap: None,
            }),
            marketing: None,
        };

        // instantiate contract
        let halo_token_contract_addr = app
            .instantiate_contract(
                halo_lp_token_contract_code_id,
                Addr::unchecked(ADMIN),
                &halo_lp_token_instantiate_msg,
                &[],
                "test instantiate contract",
                None,
            )
            .unwrap();

        // add contract info to the vector
        contract_info_vec.push(ContractInfo {
            contract_addr: halo_token_contract_addr.to_string(),
            contract_code_id: halo_lp_token_contract_code_id,
        });

        // halo reward token contract
        // create instantiate message for contract
        let halo_reward_token_instantiate_msg = Cw20InstantiateMsg {
            name: "Halo REWARD Token".to_string(),
            symbol: "HALO-RWD".to_string(),
            decimals: 18,
            initial_balances: vec![],
            mint: Some(MinterResponse {
                minter: ADMIN.to_string(),
                cap: None,
            }),
            marketing: None,
        };

        // instantiate contract
        let halo_reward_token_contract_addr = app
            .instantiate_contract(
                halo_reward_decimal_18_token_contract_code_id,
                Addr::unchecked(ADMIN),
                &halo_reward_token_instantiate_msg,
                &[],
                "test instantiate contract",
                None,
            )
            .unwrap();

        // add contract info to the vector
        contract_info_vec.push(ContractInfo {
            contract_addr: halo_reward_token_contract_addr.to_string(),
            contract_code_id: halo_reward_decimal_18_token_contract_code_id,
        });

        (app, contract_info_vec)
    }
}
