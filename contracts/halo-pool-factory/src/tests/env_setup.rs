#[cfg(test)]
pub mod env {
    use cosmwasm_std::{Addr, Coin, Empty, StdError, Uint128};
    use cw20::{Cw20Coin, MinterResponse};
    use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};

    use crate::contract::{
        execute as HaloPoolFactoryExecute, instantiate as HaloPoolFactoryInstantiate,
        query as HaloPoolFactoryQuery, reply as HaloPoolFactoryReply,
    };

    use cw20_base::contract::{
        execute as Cw20Execute, instantiate as Cw20Instantiate, query as Cw20Query,
    };

    use cw20_base::msg::{
        ExecuteMsg as Cw20ExecuteMsg, InstantiateMsg as Cw20InstantiateMsg,
        QueryMsg as Cw20QueryMsg,
    };

    use halo_pool::contract::{
        instantiate as HaloPoolInstantiate,
        execute as HaloPoolExecute,
        query as HaloPoolQuery,
    };

    use halo_pool::msg::{
        InstantiateMsg as HaloPoolInstantiateMsg,
        ExecuteMsg as HaloPoolExecuteMsg,
        QueryMsg as HaloPoolQueryMsg,
    };

    use crate::msg::{
        ExecuteMsg as HaloPoolFactoryExecuteMsg,
        InstantiateMsg as HaloPoolFactoryInstantiateMsg,
        QueryMsg as HaloPoolFactoryQueryMsg,
    };

    pub const ADMIN: &str = "aura1uh24g2lc8hvvkaaf7awz25lrh5fptthu2dhq0n";
    pub const USER_1: &str = "aura1fqj2redmssckrdeekhkcvd2kzp9f4nks4fctrt";

    pub const NATIVE_DENOM: &str = "uaura";
    pub const NATIVE_BALANCE: u128 = 1_000_000_000_000u128;

    pub const NATIVE_DENOM_2: &str = "utaura";
    pub const NATIVE_BALANCE_2: u128 = 500_000_000_000u128;

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

    fn halo_pool_factory_contract_template() -> Box<dyn Contract<Empty>> {
        let contract =
            ContractWrapper::new(HaloPoolFactoryExecute, HaloPoolFactoryInstantiate, HaloPoolFactoryQuery)
                .with_reply(HaloPoolFactoryReply);
        Box::new(contract)
    }

    fn halo_pool_contract_template() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(HaloPoolExecute, HaloPoolInstantiate, HaloPoolQuery);
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
        let halo_contract_code_id = app.store_code(halo_pool_factory_contract_template());
        let halo_lp_token_contract_code_id = app.store_code(halo_lp_token_contract_template());

        // halo pool factory contract
        // create instantiate message for contract
        let halo_pool_factory_instantiate_msg = HaloPoolFactoryInstantiateMsg {
            pool_code_id: app.store_code(halo_pool_contract_template()),
        };

        // instantiate contract
        let halo_pool_factory_contract_addr = app
            .instantiate_contract(
                halo_contract_code_id,
                Addr::unchecked(ADMIN),
                &halo_pool_factory_instantiate_msg,
                &[],
                "test instantiate contract",
                None,
            )
            .unwrap();

        // add contract info to vector
        contract_info_vec.push(ContractInfo {
            contract_addr: halo_pool_factory_contract_addr.to_string(),
            contract_code_id: halo_contract_code_id,
        });

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

        (app, contract_info_vec)
    }

}