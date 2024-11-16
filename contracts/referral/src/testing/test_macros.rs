mod macros {
   macro_rules! define_test {
        (
            name: $name:ident,
            config: {
                owner: $owner:expr,
                whitelisted_contracts: $whitelisted_contracts:expr,
                whitelisted_denoms: $whitelisted_denoms:expr
            },
            accounts: {
                $($account:ident: $balance:expr),* $(,)?
            },
            test_fn: $test_fn:expr $(,)?
        ) => {
            #[test]
            fn $name() {
                use crate::testing::test_helpers::{setup_test_env, TestEnv, create_config};
                use cosmwasm_std::testing::MockApi;

                // Set up the contract configuration
                let config = create_config (
                    $owner,
                    $whitelisted_contracts,
                    $whitelisted_denoms,
                );

                // Set up the accounts
                let accounts = vec![
                    $(
                        (MockApi::default().addr_make(stringify!($account)), $balance),
                    )*
                ];

                // Initialize the test environment
                let mut env = setup_test_env(accounts, config);

                // Execute the test function
                $test_fn(&mut env);
            }
        };
    }

   pub(crate) use define_test;
}

pub(super) use macros::define_test;
