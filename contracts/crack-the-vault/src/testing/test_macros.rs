mod macros {
   macro_rules! define_test {
        (
            name: $name:ident,
            game: {
                owner: $owner:expr,
                ticket_denom: $ticket_denom:expr,
                ticket_amount: $ticket_amount:expr,
                start_at: $start_at:expr,
                duration_seconds: $duration_seconds:expr,
                donation_addrs: $donation_addrs:expr,
                fees: $fees:expr,
            },
            accounts: {
                $($account:ident: $balance:expr),* $(,)?
            },
            test_fn: $test_fn:expr $(,)?
        ) => {
            #[test]
            fn $name() {
                use crate::testing::test_helpers::{setup_test_env, TestEnv, create_partial_instantiate};
                use cosmwasm_std::testing::MockApi;

                // Set up the contract competition configuration
                let config = create_partial_instantiate (
                    $owner,
                    $ticket_denom,
                    $ticket_amount,
                    $start_at,
                    $duration_seconds,
                    $donation_addrs,
                    $fees,
                );

                // Set up the accounts
                let accounts = vec![
                    $(
                        (MockApi::default().addr_make(stringify!($account)), $balance),
                    )*
                ];

                // Initialize the test environment
                let mut env = setup_test_env( accounts, config);

                // Execute the test function
                $test_fn(&mut env);
            }
        };
    }

   pub(crate) use define_test;
}

pub(super) use macros::define_test;