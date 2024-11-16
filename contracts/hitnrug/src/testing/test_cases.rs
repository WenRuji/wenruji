use crate::{config::ConfigUpdate, game::GameSmSnapshot, msg::QueryMsg};
use cosmwasm_std::{coin, coins, Decimal, Timestamp, Uint128};

use super::test_macros::define_test;

define_test! {
    name: test_lifecycle,
    game: {
        // Standard game configuration
        owner: "owner",
        ticket_denom: "denom",
        ticket_amount: Uint128::new(100),
        start_at: Timestamp::from_seconds(1000),
        duration_seconds: 600u64,
        delay_play_seconds: 60u64,
        fees: vec![
            Decimal::percent(10),  // 10% fee for owner
            Decimal::percent(10),  // 10% referral fee
        ],
        pt_keep: 4i64,
        pt_hit: -5i64,
        pt_help: (6i64, 4i64),
    },
    accounts: {
        alice: coins(200u128, "denom"),
        bob: coins(200u128, "denom"),
        charlie: coins(200u128, "denom"),
    },
    test_fn: |env: &mut TestEnv| {
        // Initialize block timestamp before to start the game
        env.set_block(Timestamp::from_seconds(999));

        // **Join Phase**
        // Standard joining process for three players

        env.join("alice", None, coins(100, "denom")).unwrap();   // Alice joins the game
        env.join("bob", None, coins(100, "denom")).unwrap();  // Bob joins
        env.join("charlie", None, coins(100, "denom")).unwrap(); // Charlie joins without referral

        // Initialize block timestamp to start the game
        env.set_block(Timestamp::from_seconds(1000));


        // **Gameplay Actions**
        // Players take standard actions to adjust scores

        env.play_keep("alice").unwrap(); // Alice plays 'keep'; Alice score = 4

        env.play_help("charlie", "alice").unwrap(); // Charlie hits alice; Charlie score = 0, Bob score = 2

        env.set_block(Timestamp::from_seconds(1061));

        // **Mid-game Status Check**
        // Move forward in time and check if players can continue actions
        env.play_keep("alice").unwrap(); // Alice plays 'keep'; Alice score = 5

        env.play_keep("charlie").unwrap(); // Alice plays 'keep'; Alice score = 5

        // **Endgame**
        // Move to game end and finalize results

        env.set_block(Timestamp::from_seconds(1601)); // Move to end of game duration

        env.verify_winner("alice"); // verify winner is Alice
        env.endgame("anyone").unwrap(); // Game is ended, distributing rewards

        // **Balance Verification**
        // Verify that rewards, fees, and balances are correctly allocated

        env.assert_balance("alice", coin(366u128, "denom"));    // Winner Alice takes 240
        env.assert_balance("bob", coin(100u128, "denom"));      // Initial entry fee not refunded for Bob
        env.assert_balance("charlie", coin(100u128, "denom"));  // Initial entry fee not refunded for Charlie

        // **Owner and Referral Fees**
        env.assert_balance("owner", coin(33u128, "denom"));  // Owner receives 10% of pot
        // let referral_balances = env.app.wrap().query_all_balances(env.contracts.referral.clone()).unwrap();
        // assert_eq!(referral_balances, coins(30, "denom")); // Referral contract receives 10% of pot

        // Save the Game Snapshot
        let game_snap = env.get_snap(None);

        // **Game Restart**
        // Restart the game to reset for a new round

        env.restart("anyone").unwrap(); // Game restarts for the next round

        // **Verify Snapshots**
        let game_snap_old = env.get_snap(Some(1));
        assert_eq!(game_snap, game_snap_old);
        assert_ne!(game_snap, env.get_snap(None))

    }
}

define_test! {
    name: test_join_edge_cases,
    game: {
        owner: "owner",
        ticket_denom: "denom",
        ticket_amount: Uint128::new(100),
        start_at: Timestamp::from_seconds(1000),
        duration_seconds: 600u64,
        delay_play_seconds: 60u64,
        fees: vec![
            Decimal::percent(10),
            Decimal::percent(10),
        ],
        pt_keep: 5i64,
        pt_hit: -6i64,
        pt_help: (8i64, 4i64),
    },
    accounts: {
        alice: coins(200u128, "denom"),
        bob: coins(200u128, "denom"),
        charlie: coins(200u128, "denom"),
        VALID_USER: vec![
            coin(200u128, "denom"),
            coin(200u128, "donate")
        ],
    },
    test_fn: |env: &mut TestEnv| {
        env.set_block(Timestamp::from_seconds(999));

        // Join with invalid denom
        env.join("alice", None, coins(100, "wrong_denom")).unwrap_err();

        // Join with sufficient denom but invalid referral code
        env.join("bob", Some("INVALID_CODE".to_string()), coins(100, "denom")).unwrap_err();

        // Attempt to re-join (should throw error)
        //env.join("bob", Some("VALID_CODE".to_string()), coins(100, "denom")).unwrap();
        env.join("bob", None, coins(100, "denom")).unwrap();

        // Join with insufficient funds
        env.join("charlie", None, coins(50, "denom")).unwrap_err(); // less than required entry fee

        env.set_block(Timestamp::from_seconds(1000));
        // Join after start (should throw error)
        env.join("bob", Some("VALID_CODE".to_string()), coins(100, "denom")).unwrap_err();

    }
}

define_test! {
    name: test_play_with_timing,
    game: {
        owner: "owner",
        ticket_denom: "denom",
        ticket_amount: Uint128::new(100),
        start_at: Timestamp::from_seconds(1000),
        duration_seconds: 600u64,
        delay_play_seconds: 60u64,
        fees: vec![
            Decimal::percent(10),
            Decimal::percent(10),
        ],
        pt_keep: 5i64,
        pt_hit: -6i64,
        pt_help: (8i64, 4i64),
    },
    accounts: {
        alice: coins(200u128, "denom"),
        bob: coins(200u128, "denom"),
        charlie: coins(200u128, "denom"),
        VALID_USER: vec![
            coin(200u128, "denom"),
            coin(200u128, "donate")
        ],
    },
    test_fn: |env: &mut TestEnv| {
        env.set_block(Timestamp::from_seconds(999));

        env.join("alice", None, coins(100, "denom")).unwrap();
        env.join("bob", None, coins(100, "denom")).unwrap();

        env.set_block(Timestamp::from_seconds(1000));

        // Alice plays and then tries to play again before 60s delay
        env.play_keep("alice").unwrap();
        env.play_keep("alice").unwrap_err(); // should fail due to delay

        // Set block time to allow alice to play again
        env.set_block(Timestamp::from_seconds(1061));
        env.play_keep("alice").unwrap(); // should now succeed

        // Alice exits and then tries to play again
        env.exit("alice").unwrap();
        env.play_keep("alice").unwrap_err(); // should fail due to exit
        env.play_help("alice", "bob").unwrap_err(); // should fail due to exit
        env.play_hit("alice", "bob").unwrap_err(); // should fail due to exit
    }
}

define_test! {
    name: test_exit_scenarios,
    game: {
        owner: "owner",
        ticket_denom: "denom",
        ticket_amount: Uint128::new(100),
        start_at: Timestamp::from_seconds(1000),
        duration_seconds: 600u64,
        delay_play_seconds: 60u64,
        fees: vec![
            Decimal::percent(10),
            Decimal::percent(10),
        ],
        pt_keep: 5i64,
        pt_hit: -6i64,
        pt_help: (8i64, 4i64),
    },
    accounts: {
        alice: coins(200u128, "denom"),
        bob: coins(200u128, "denom"),
        charlie: coins(200u128, "denom"),
        VALID_USER: vec![
            coin(200u128, "denom"),
            coin(200u128, "donate")
        ],
    },
    test_fn: |env: &mut TestEnv| {
        env.set_block(Timestamp::from_seconds(999));

        env.join("alice", None, coins(100, "denom")).unwrap();
        env.join("bob", None, coins(100, "denom")).unwrap();

        env.set_block(Timestamp::from_seconds(1000));

        // Alice exits at the beginning, should receive full refund
        env.exit("alice").unwrap();
        env.assert_balance("alice", coin(200u128, "denom")); // 100 from joining, 100 refund

        // Try to exit again (should throw error)
        env.exit("alice").unwrap_err();

        // Mid-game exit with partial refund
        env.set_block(Timestamp::from_seconds(1300)); // halfway through game
        env.exit("bob").unwrap();
        env.assert_balance("bob", coin(150u128, "denom")); // 50% of ticket_amount refunded
    }
}

define_test! {
    name: test_endgame_conditions,
    game: {
        owner: "owner",
        ticket_denom: "denom",
        ticket_amount: Uint128::new(100),
        start_at: Timestamp::from_seconds(1000),
        duration_seconds: 600u64,
        delay_play_seconds: 60u64,
        fees: vec![
            Decimal::percent(10),
            Decimal::percent(10),
        ],
        pt_keep: 5i64,
        pt_hit: -6i64,
        pt_help: (8i64, 4i64),
    },
    accounts: {
        alice: coins(200u128, "denom"),
        bob: coins(200u128, "denom"),
        charlie: coins(200u128, "denom"),
        VALID_USER: vec![
            coin(200u128, "denom"),
            coin(200u128, "donate")
        ],
    },
    test_fn: |env: &mut TestEnv| {
        env.set_block(Timestamp::from_seconds(999));

        env.join("alice", None, coins(100, "denom")).unwrap();
        env.join("bob", None, coins(100, "denom")).unwrap();

        env.set_block(Timestamp::from_seconds(1000));

        // Attempt to end the game before time is up
        env.endgame("alice").unwrap_err(); // should fail as game hasn't ended yet

        // Move block time to game end and try ending again
        env.set_block(Timestamp::from_seconds(1601));
        env.endgame("alice").unwrap();

        // Check balances for proper fee and prize distribution
        env.assert_balance("owner", coin(22u128, "denom")); // 10% fee for owner from the prize + 10% from the referral
        let balances = env.app.wrap().query_all_balances(env.contracts.referral.clone()).unwrap();
        assert_eq!(balances, []); // No one has used referral

        // Verify winner receives remaining prize pot
        // Winner is the first to join if no points are made
        // She gets 80% of 200 => 160 + 90% of referrals 18 => 178
        // there is a small rounding error because its actually calculated as 90% as total 10% of 90% and 90% of 90%
        env.assert_balance("alice", coin(277u128, "denom"));
    }
}

define_test! {
    name: test_restart_game,
    game: {
        owner: "owner",
        ticket_denom: "denom",
        ticket_amount: Uint128::new(100),
        start_at: Timestamp::from_seconds(1000),
        duration_seconds: 600u64,
        delay_play_seconds: 60u64,
        fees: vec![
            Decimal::percent(10),  // 10% fee for owner
            Decimal::percent(10),  // 10% referral fee
        ],
        pt_keep: 5i64,
        pt_hit: -6i64,
        pt_help: (8i64, 4i64),
    },
    accounts: {
        alice: coins(2000u128, "denom"),
        bob: coins(2000u128, "denom"),
        charlie: coins(2000u128, "denom"),
        VALID_USER: vec![
            coin(200u128, "denom"),
            coin(200u128, "donate"),
        ],
    },
    test_fn: |env: &mut TestEnv| {
        env.set_block(Timestamp::from_seconds(999));

        env.join("alice", None, coins(100, "denom")).unwrap();

        // Attempt to restart the game before it has ended (should fail)
        env.restart("anyone").unwrap_err();

        // End the game and prepare for restart
        env.set_block(Timestamp::from_seconds(1601));
        env.endgame("alice").unwrap();

        // Restart game successfully after it has ended
        env.restart("anyone").unwrap();

        // Loop to restart the game over 10 rounds, checking snapshots
        for i in 1..=10 {
            env.join("alice", None, coins(100, "denom")).unwrap();

            let time = 1601 + i * 600 +i+i*10 ;
            env.set_block(Timestamp::from_seconds(time));
            env.endgame("alice").unwrap();

            // Restart game successfully after it has ended
            env.restart("anyone").unwrap();
        }

        // Verify state reset - no players in the game
        env.join("alice", None, coins(100, "denom")).unwrap();

        // Verify Snap 1 got erased and snap 11 exist
        env.get_snap(Some(11));
        env.app.wrap()
        .query_wasm_smart::<GameSmSnapshot>(env.contracts.game.clone(), &QueryMsg::GameStatus { idx: Some(1) })
        .unwrap_err()
    }
}

define_test! {
    name: test_small_tickets_and_boundary_timings,
    game: {
        owner: "owner",
        ticket_denom: "denom",
        ticket_amount: Uint128::new(1), // very small ticket amount
        start_at: Timestamp::from_seconds(1000),
        duration_seconds: 600u64,
        delay_play_seconds: 60u64,
        fees: vec![
            Decimal::percent(10),
            Decimal::percent(10),
        ],
        pt_keep: 5i64,
        pt_hit: -6i64,
        pt_help: (8i64, 4i64),
    },
    accounts: {
        tiny_tickets: coins(2u128, "denom"), // low balance for edge case
    },
    test_fn: |env: &mut TestEnv| {
        env.set_block(Timestamp::from_seconds(999));

        env.join("tiny_tickets", None, coins(1, "denom")).unwrap();
        env.assert_balance("tiny_tickets", coin(1u128, "denom")); // should have 1 left after joining

        env.set_block(Timestamp::from_seconds(1000));

        // Attempt to play exactly at start time
        env.play_keep("tiny_tickets").unwrap();
        env.set_block(Timestamp::from_seconds(1060)); // exactly delay time
        env.play_keep("tiny_tickets").unwrap();  // should be able to play at boundary
    }
}

define_test! {
    name: test_update_config,
    game: {
        owner: "owner",
        ticket_denom: "denom",
        ticket_amount: Uint128::new(1),
        start_at: Timestamp::from_seconds(1000),
        duration_seconds: 600u64,
        delay_play_seconds: 60u64,
        fees: vec![
            Decimal::percent(10),
            Decimal::percent(10),
        ],
        pt_keep: 5i64,
        pt_hit: -6i64,
        pt_help: (8i64, 4i64),
    },
    accounts: {
        tiny_tickets: coins(2u128, "denom"),
    },
    test_fn: |env: &mut TestEnv| {
        env.set_block(Timestamp::from_seconds(1000));
        let old_config = env.get_config();
        let new_config = ConfigUpdate {
            owner: Some(env.addr("new_owner")),
            ticket_denom: None,
            ticket_amount: None,
            duration_seconds: None,
            delay_play_seconds: None,
            game_delay_sec: None,
            fees: None,
            points: None
        };

        env.update_config("owner", new_config.clone()).unwrap_err(); //error the game should be finished

        env.set_block(Timestamp::from_seconds(1601)); // endgame timestamp
        env.endgame("account").unwrap_err(); // no needed because no player

        env.update_config("wrong_owner", new_config.clone()).unwrap_err(); //error owner can change only
        env.update_config("owner", new_config.clone()).unwrap(); // Update Should work

        let config = env.get_config();
        assert_ne!(old_config, config);
        assert_eq!(new_config.owner.unwrap(), config.owner);
    }
}
