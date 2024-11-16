use cosmwasm_std::{coin, coins, to_json_string, Addr, Decimal};

use crate::msg::*;

use super::test_macros::define_test;

define_test! {
    name: test_gen_code,
    config: {
        owner: "owner",
        whitelisted_contracts: Whitelist::All,
        whitelisted_denoms: Whitelist::All
    },
    accounts: {
        alice: coins(1000, "utoken"),
    },
    test_fn: |env: &mut TestEnv| {
        env.gen_code("alice", "CODE".to_string()).unwrap();
        env.assert_code("alice", "CODE".to_string());
    }
}

define_test! {
    name: test_add_referee,
    config: {
        owner: "owner",
        whitelisted_contracts: Whitelist::All,
        whitelisted_denoms: Whitelist::All
    },
    accounts: {
        alice: coins(1000, "utoken"),
        bob: coins(500, "utoken"),
    },
    test_fn: |env: &mut TestEnv| {
        env.gen_code("alice", "CODE".to_string()).unwrap();
        env.add_referee("bob", "alice" ,"CODE".to_string()).unwrap();
        //env.assert_referee("bob", "alice".to_string());
    }
}

define_test! {
    name: test_update_config,
    config: {
        owner: "owner",
        whitelisted_contracts: Whitelist::All,
        whitelisted_denoms: Whitelist::All
    },
    accounts: {
        alice: coins(1000, "utoken"),
    },
    test_fn: |env: &mut TestEnv| {
        let new_config = ConfigUpdate {
            owner: Some(env.addr("new_owner")),
            whitelisted_contracts: Some(Whitelist::Some(vec![])),
            whitelisted_denoms: Some(Whitelist::Some(vec![])),
        };
        env.update_config("owner", new_config).unwrap();
        //env.assert_config("new_owner".to_string(), Whitelist::Some(vec![]), Whitelist::Some(vec![]));
    }
}

define_test! {
    name: test_distribute_rewards,
    config: {
        owner: "owner",
        whitelisted_contracts: Whitelist::All,
        whitelisted_denoms: Whitelist::All
    },
    accounts: {
        owner: coins(2000, "utoken"),
        alice: coins(1000, "utoken"),
        bob: coins(500, "utoken"),
    },
    test_fn: |env: &mut TestEnv| {
        let rewards = coins(200, "utoken");
        let fees = vec![
            (env.addr("alice"), Decimal::one()),
            (env.addr("bob"), Decimal::one()),
        ];
        env.distribute_rewards("owner", rewards, fees).unwrap();
        env.assert_pending_rewards("alice", coins(100, "utoken"));
        env.assert_pending_rewards("bob", coins(100, "utoken"));
    }
}

define_test! {
    name: test_claim_rewards,
    config: {
        owner: "owner",
        whitelisted_contracts: Whitelist::All,
        whitelisted_denoms: Whitelist::All
    },
    accounts: {
        alice: coins(1000, "utoken"),
        owner: coins(1000, "urewards")
    },
    test_fn: |env: &mut TestEnv| {
        let rewards = vec![coin(1000, "urewards")];
        let ambassador = vec![
            (env.addr("alice"), Decimal::one())];
        env.distribute_rewards("owner", rewards, ambassador).unwrap();
        env.claim_rewards("alice").unwrap();
        env.assert_balance("alice", coin(1000, "urewards"));
    }
}

define_test! {
    name: test_claim_rewards_without_accrual,
    config: {
        owner: "owner",
        whitelisted_contracts: Whitelist::All,
        whitelisted_denoms: Whitelist::All
    },
    accounts: {
        alice: coins(1000, "utoken"),
    },
    test_fn: |env: &mut TestEnv| {
        let result = env.claim_rewards("alice");
        assert!(result.is_err(), "Should not be able to claim rewards without accrual");
    }
}

define_test! {
    name: test_whitelisted_contract_only_alice,
    config: {
        owner: "owner",
        whitelisted_contracts: Whitelist::Some(vec![]),
        whitelisted_denoms: Whitelist::All
    },
    accounts: {
        alice: coins(1000, "utoken"),
        bob: coins(500, "utoken"),
    },
    test_fn: |env: &mut TestEnv| {
        env.update_config("owner", ConfigUpdate {whitelisted_contracts:Some(Whitelist::Some(vec![env.addr("alice").to_string()])), owner: None, whitelisted_denoms: None }).unwrap();
        // Alice should be able to add referee and distribute rewards
        env.gen_code("alice", "CODE".to_string()).unwrap();
        env.add_referee("alice", "bob", "CODE".to_string()).unwrap();
        let rewards = vec![coin(100, "utoken")];
        let ambassador = vec![(env.addr("alice"), Decimal::one())];
        env.distribute_rewards("alice", rewards.clone(), ambassador.clone()).unwrap();

        // Bob should not be able to add referee or distribute rewards
        env.add_referee("bob", "alice", "CODE".to_string()).unwrap_err();
        env.distribute_rewards("bob", rewards, ambassador).unwrap_err();
    }
}

define_test! {
    name: test_whitelisted_denom,
    config: {
        owner: "owner",
        whitelisted_contracts: Whitelist::All,
        whitelisted_denoms: Whitelist::Some(vec!["utoken".to_string()])
    },
    accounts: {
        alice: coins(1000, "utoken"),
        owner: coins(1000, "utoken")
    },
    test_fn: |env: &mut TestEnv| {
        // Distributing with whitelisted denom
        let rewards = vec![coin(100, "utoken")];
        let ambassador = vec![(env.addr("alice"), Decimal::one())];
        env.distribute_rewards("owner", rewards.clone(), ambassador.clone()).unwrap();

        // Distributing with non-whitelisted denom
        let non_whitelisted_rewards = vec![coin(100, "nonwhitelisted")];
        env.distribute_rewards("owner", non_whitelisted_rewards, ambassador).unwrap_err();
    }
}

define_test! {
    name: test_non_owner_cannot_update_config,
    config: {
        owner: "owner",
        whitelisted_contracts: Whitelist::All,
        whitelisted_denoms: Whitelist::All
    },
    accounts: {
        alice: coins(1000, "utoken"),
        bob: coins(500, "utoken"),
    },
    test_fn: |env: &mut TestEnv| {
        let new_config = ConfigUpdate {
            owner: Some(env.addr("new_owner")),
            whitelisted_contracts: Some(Whitelist::All),
            whitelisted_denoms: Some(Whitelist::All),
        };
        // Owner can update config
        env.update_config("owner", new_config.clone()).unwrap();

        env.update_config("alice", new_config.clone()).unwrap_err();
    }
}

define_test! {
    name: test_add_referee_with_non_existing_code,
    config: {
        owner: "owner",
        whitelisted_contracts: Whitelist::All,
        whitelisted_denoms: Whitelist::All
    },
    accounts: {
        alice: coins(1000, "utoken"),
        bob: coins(500, "utoken"),
    },
    test_fn: |env: &mut TestEnv| {
        // Attempt to add referee with a non-existing code
        env.add_referee("bob", "alice", "NONEXISTENT".to_string()).unwrap_err();

        // Generate a code and add referee successfully
        env.gen_code("alice", "CODE".to_string()).unwrap();
        env.add_referee("bob", "alice", "CODE".to_string()).unwrap();

        // Attempt to add the same referee again should result in an error
        env.add_referee("bob", "alice", "CODE".to_string()).unwrap_err();
    }
}

#[test]
fn test_instantiate_msg_json() {
   // Assuming Whitelist is a Vec<Addr> or a similar structure
   let instance = InstantiateMsg {
      owner: Addr::unchecked("owner_address"),
      whitelisted_denoms: Whitelist::All,
      whitelisted_contracts: Whitelist::All,
   };

   // Serialize the instance to JSON and print it
   let json_instance = to_json_string(&instance).unwrap();
   println!("{}", json_instance);
}
