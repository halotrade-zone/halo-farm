#[cfg(test)]
pub mod env {
    use cosmwasm_std::{Addr, Coin, Empty, StdError, Uint128};
    use cw20::{Cw20Coin, MinterResponse};
    use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};

    use crate::contract::{
        execute as HaloPoolFactoryExecute, instantiate as HaloPoolFactoryInstantiate,
        query as HaloPoolFactoryQuery, reply as HaloPoolFactoryReply,
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

}