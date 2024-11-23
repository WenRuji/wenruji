use cosmwasm_std::{coin, coins, Decimal, Timestamp, Uint128};

use super::test_macros::define_test;

define_test! {
    name: test_lifecycle,
    game: {
        owner: "owner",
        ticket_denom: "kujira15drytn4ntvg7f292ncul6wcxle4xe404q280hcw9878zjqa2h9nqulj2e9",
        ticket_amount: Uint128::new(100),
        start_at: Timestamp::from_seconds(1000),
        duration_seconds: 1000u64,
        donation_addrs: vec![
            MockApi::default().addr_make("donald")
        ],
        fees: vec![
            Decimal::percent(10),
            Decimal::percent(10),
            Decimal::percent(10)
        ],
    },
    accounts: {
        alice: coins(200u128, "kujira15drytn4ntvg7f292ncul6wcxle4xe404q280hcw9878zjqa2h9nqulj2e9"),
        bob: coins(200u128, "kujira15drytn4ntvg7f292ncul6wcxle4xe404q280hcw9878zjqa2h9nqulj2e9"),
        charlie: coins(200u128, "kujira15drytn4ntvg7f292ncul6wcxle4xe404q280hcw9878zjqa2h9nqulj2e9"),
        donald: vec![
            coin(200u128, "kujira15drytn4ntvg7f292ncul6wcxle4xe404q280hcw9878zjqa2h9nqulj2e9"),
            coin(200u128, "donate")
        ],
        VALID_USER: vec![
            coin(200u128, "kujira15drytn4ntvg7f292ncul6wcxle4xe404q280hcw9878zjqa2h9nqulj2e9"),
            coin(200u128, "donate")
        ],
    },
    test_fn: |env: &mut TestEnv| {
        //clean the block timestamp
        env.set_block(Timestamp::from_seconds(1000));

        //Test Join
        env.join("alice", None, coins(100, "kujira15drytn4ntvg7f292ncul6wcxle4xe404q280hcw9878zjqa2h9nqulj2e9")).unwrap();
        env.join("alice", None, coins(100, "kujira15drytn4ntvg7f292ncul6wcxle4xe404q280hcw9878zjqa2h9nqulj2e9")).unwrap_err(); //rejoin err
        env.join("bob", Some("NOT_VALID_CODE".to_string()), coins(100, "kujira15drytn4ntvg7f292ncul6wcxle4xe404q280hcw9878zjqa2h9nqulj2e9")).unwrap_err(); //join with Not valid code error
        env.join("bob", Some("VALID_CODE".to_string()), coins(100, "kujira15drytn4ntvg7f292ncul6wcxle4xe404q280hcw9878zjqa2h9nqulj2e9")).unwrap(); //join with valid code
        env.join("VALID_USER", None, coins(100, "donate")).unwrap_err(); //join with wrong kujira15drytn4ntvg7f292ncul6wcxle4xe404q280hcw9878zjqa2h9nqulj2e9 error
        env.join("VALID_USER", None, coins(100, "kujira15drytn4ntvg7f292ncul6wcxle4xe404q280hcw9878zjqa2h9nqulj2e9")).unwrap(); //join with user that already has been linked to someone VALID_AMBASSADOR

        // JOINED IN THE GAME: alice - bob - VALID_USER

        // Test Donations
        env.donate("donald", vec![coin(100u128, "kujira15drytn4ntvg7f292ncul6wcxle4xe404q280hcw9878zjqa2h9nqulj2e9"), coin(100u128, "donate")]).unwrap(); //success he's whitelisted
        env.donate("alice", vec![coin(10u128, "kujira15drytn4ntvg7f292ncul6wcxle4xe404q280hcw9878zjqa2h9nqulj2e9")]).unwrap_err(); //error alice is not whitelisted

        // Test Exit
        env.exit("donald").unwrap_err(); // he didnt join
        env.exit("alice").unwrap();
        env.assert_balance("alice", coin(200u128, "kujira15drytn4ntvg7f292ncul6wcxle4xe404q280hcw9878zjqa2h9nqulj2e9")); // she gets everything back because the time is not moved

        env.move_block(500);
        env.exit("bob").unwrap();
        env.assert_balance("bob", coin(150u128, "kujira15drytn4ntvg7f292ncul6wcxle4xe404q280hcw9878zjqa2h9nqulj2e9")); // he gets only 50u128 due to the competition time half time.

        env.move_block(500);
        env.exit("VALID_USER").unwrap_err(); //he doesn't get anything back
        env.assert_balance("VALID_USER", coin(100u128, "kujira15drytn4ntvg7f292ncul6wcxle4xe404q280hcw9878zjqa2h9nqulj2e9"));

        env.exit("alice").unwrap_err(); //try to cheat exiting to times => err

        //Test restart
        env.restart("alice").unwrap_err(); //competition not ended

        //TEST END
        env.set_block(Timestamp::from_seconds(1500)); //go back to test error if endgame before the time
        env.endgame("owner", "alice", false).unwrap_err(); //not ended

        env.set_block(Timestamp::from_seconds(2001)); //end competition
        env.endgame("not_owner", "alice", false).unwrap_err(); //error only owner can set the winner

        env.endgame("owner", "alice", true).unwrap();

        env.set_block(Timestamp::from_seconds(4001)); //go back to test error if endgame before the time
        env.endgame("owner", "alice", true).unwrap();

        env.join("alice", None, coins(100, "kujira15drytn4ntvg7f292ncul6wcxle4xe404q280hcw9878zjqa2h9nqulj2e9")).unwrap();
    }
}
