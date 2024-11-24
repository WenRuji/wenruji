use cosmwasm_std::{
   ensure, wasm_execute, Addr, Api, BankMsg, Coin, Decimal, Empty, Order, QuerierWrapper, Response,
   Storage, Timestamp, Uint128, WasmMsg,
};
use cw_storage_plus::{Item, Map};
use referral::{ExecuteMsg, QueryMsg};
use wenruji_rs::{
   calculate_fee_distribution, normalize, to_addr, DecayGame, DecayGameAccount, DecayGameError,
};

use crate::{config::Config, ContractError};

pub static DECAY_GAME: Item<DecayGame> = Item::new("dg");
pub static ACCOUNTS: Map<Addr, DecayGameAccount> = Map::new("dg/a");
pub static REWARDS: Map<String, Coin> = Map::new("r");
pub static REF_WEIGHTS: Map<Addr, Decimal> = Map::new("rw");
pub static ADMIN: Map<&Addr, Empty> = Map::new("admin");

pub fn execute_ref(
   api: &dyn Api,
   storage: &mut dyn Storage,
   querier: QuerierWrapper,
   config: &Config,
   account: &Addr,
   ref_code: Option<String>,
) -> Result<(Option<String>, Option<WasmMsg>), ContractError> {
   let ambassador: String = querier.query_wasm_smart(
      config.contracts.referral.clone(),
      &QueryMsg::GetReferrer { user: account.clone() },
   )?;
   if !ambassador.is_empty() {
      let ambassador_addr = to_addr(ambassador.clone(), api)?;
      let ref_weight =
         REF_WEIGHTS.may_load(storage, ambassador_addr.clone())?.unwrap_or(Decimal::zero());
      REF_WEIGHTS.save(storage, ambassador_addr, &(ref_weight + Decimal::one()))?;

      return Ok((Some(ambassador), None));
   } else {
      if let Some(ref_code) = ref_code {
         // Query the referral contract to get the referrer address
         let ambassador: String = querier.query_wasm_smart(
            config.contracts.referral.clone(),
            &QueryMsg::GetAddr { code: ref_code.clone() },
         )?;
         let ambassador_addr = to_addr(ambassador.clone(), api)?;
         let ref_weight =
            REF_WEIGHTS.may_load(storage, ambassador_addr.clone())?.unwrap_or(Decimal::zero());
         REF_WEIGHTS.save(storage, ambassador_addr, &(ref_weight.checked_add(Decimal::one())?))?;

         let msg = wasm_execute(
            config.contracts.referral.clone(),
            &ExecuteMsg::AddReferee { referee: account.clone(), code: ref_code },
            vec![],
         )?;
         return Ok((Some(ambassador.clone()), Some(msg)));
      }
      return Ok((None, None));
   }
}

pub fn execute_join(
   storage: &mut dyn Storage,
   now: Timestamp,
   account: &Addr,
   amount: Uint128,
) -> Result<(), ContractError> {
   let mut decay_game = DECAY_GAME.load(storage)?;
   let account_data = decay_game.join(amount, &now)?;
   DECAY_GAME.save(storage, &decay_game)?;
   ACCOUNTS.save(storage, account.clone(), &account_data)?;
   Ok(())
}

pub fn execute_exit(
   storage: &mut dyn Storage,
   now: Timestamp,
   account: &Addr,
) -> Result<(Uint128, Decimal), ContractError> {
   let mut decay_game = DECAY_GAME.load(storage)?;

   ensure!(now.lt(&decay_game.decay_ends_at), ContractError::Invalid("game_ended".to_string()));

   let mut account_data = ACCOUNTS.load(storage, account.clone())?;

   if !account_data.decay_snapshot.is_zero() {
      return Err(ContractError::Invalid("already_exited".to_string()));
   }

   decay_game.exit(&now, &mut account_data);

   let amount = decay_game.claim(&mut account_data);

   DECAY_GAME.save(storage, &decay_game)?;
   ACCOUNTS.save(storage, account.clone(), &account_data)?;

   Ok((amount, account_data.decay_snapshot))
}

pub fn execute_endgame(
   storage: &mut dyn Storage,
   now: Timestamp,
) -> Result<Uint128, ContractError> {
   let mut decay_game = DECAY_GAME.load(storage)?;
   let amount = decay_game.distribute_rewards(&now)?;
   DECAY_GAME.save(storage, &decay_game)?;
   Ok(amount)
}

pub fn execute_donate(storage: &mut dyn Storage, coin: Coin) -> Result<(), ContractError> {
   let old_coin = REWARDS
      .load(storage, coin.denom.clone())
      .unwrap_or(Coin { denom: coin.denom.clone(), amount: Uint128::zero() });

   REWARDS.save(
      storage,
      coin.denom.clone(),
      &Coin { denom: old_coin.denom, amount: old_coin.amount + coin.amount },
   )?;
   Ok(())
}

pub fn execute_restart(
   storage: &mut dyn Storage,
   now: Timestamp,
   config: &Config,
) -> Result<(), ContractError> {
   let decay_game = DECAY_GAME.load(storage)?;
   if now.le(&decay_game.decay_ends_at) {
      return Err(ContractError::DecayGameError(DecayGameError::DecayNotEnded {}));
   }
   if !(decay_game.rewards == decay_game.total - decay_game.exited) {
      return Err(ContractError::GameNotEnded {});
   }
   ACCOUNTS.clear(storage);
   REF_WEIGHTS.clear(storage);
   DECAY_GAME.remove(storage);
   let start_time = now.plus_seconds(config.game_delay);
   let decay_game = DecayGame::new(start_time, start_time.plus_seconds(config.duration_seconds));
   DECAY_GAME.save(storage, &decay_game)?;
   Ok(())
}

pub fn execute_post_swap(
   storage: &mut dyn Storage,
   config: &Config,
   winner: Addr,
   coin: Coin,
) -> Result<Response, ContractError> {
   let mut response = Response::new();

   let mut fees = config.fees.clone();
   fees.insert(0, (winner, config.winner_share));
   let fee_split = calculate_fee_distribution(vec![coin], &fees);

   // Collect all existing rewards, merge with winner rewards, and normalize
   let rewards: Vec<Coin> = REWARDS
      .range(storage, None, None, Order::Ascending)
      .filter_map(|item| item.ok())
      .map(|(_, coin)| coin)
      .chain(fee_split[0].1.clone()) // Winner’s rewards
      .collect();

   // Dispatch messages to send coins and distribute referral rewards
   response = response.add_messages(vec![
      BankMsg::Send { to_address: fee_split[0].0.to_string(), amount: normalize(rewards) },
      BankMsg::Send { to_address: fee_split[1].0.to_string(), amount: fee_split[1].1.clone() },
      BankMsg::Send { to_address: fee_split[2].0.to_string(), amount: fee_split[2].1.clone() },
   ]);

   // Collect referral weights and trigger referral reward distribution
   let referrals = REF_WEIGHTS
      .range(storage, None, None, Order::Ascending)
      .filter_map(|item| item.ok())
      .collect();

   response = response.add_message(wasm_execute(
      config.contracts.referral.clone(),
      &ExecuteMsg::DistributeRewards { referers: referrals },
      fee_split[3].1.clone(),
   )?);

   REWARDS.clear(storage);
   Ok(response)
}

#[cfg(test)]
mod tests {
   use crate::msg::Contracts;

   use super::*;
   use cosmwasm_std::{
      testing::mock_dependencies, Addr, BankMsg, Coin, Decimal, Timestamp, Uint128,
   };

   fn setup_config() -> Config {
      Config {
         owner: Addr::unchecked("owner"),
         ticket_denom: "utoken".to_string(),
         ticket_amount: Uint128::new(100),
         duration_seconds: 3600,
         contracts: Contracts {
            referral: Addr::unchecked("referral_contract"),
            swap: Addr::unchecked("swap_contract"),
         },
         donation_addrs: vec![Addr::unchecked("donation1")],
         fees: vec![
            (Addr::unchecked("platform"), Decimal::percent(1)),
            (Addr::unchecked("nami"), Decimal::percent(2)),
            (Addr::unchecked("referral"), Decimal::percent(1)),
         ],
         winner_share: Decimal::percent(90),
         game_delay: 60u64,
      }
   }

   #[test]
   fn test_execute_join() {
      let mut deps = mock_dependencies();
      let now = Timestamp::from_seconds(1_000_000);
      let account = Addr::unchecked("player1");
      let amount = Uint128::new(1000);

      // Initialize DecayGame in storage
      DECAY_GAME.save(&mut deps.storage, &DecayGame::new(now, now.plus_seconds(3600))).unwrap();

      // Execute join
      let result = execute_join(&mut deps.storage, now, &account, amount);
      assert!(result.is_ok());

      // Verify that the account was saved in storage
      let account_data = ACCOUNTS.load(&deps.storage, account).unwrap();
      assert_eq!(account_data.amount, amount);
   }

   #[test]
   fn test_execute_exit() {
      let mut deps = mock_dependencies();
      let now = Timestamp::from_seconds(1_000_000);
      let account = Addr::unchecked("player1");
      let amount = Uint128::new(1000);

      // Initialize DecayGame and Account data in storage
      DECAY_GAME.save(&mut deps.storage, &DecayGame::new(now, now.plus_seconds(3600))).unwrap();
      ACCOUNTS
         .save(
            &mut deps.storage,
            account.clone(),
            &DecayGameAccount { amount, decay_snapshot: Decimal::zero(), pending: Uint128::zero() },
         )
         .unwrap();

      // Execute exit error now game ended
      execute_exit(&mut deps.storage, now.plus_days(1), &account).unwrap_err();

      // Execute exit
      let result = execute_exit(&mut deps.storage, now, &account);
      assert!(result.is_ok());

      // Execute exit error already claimed
      execute_exit(&mut deps.storage, now, &account).unwrap_err();

      // Verify that the account's decay_snapshot was set
      let account_data = ACCOUNTS.load(&deps.storage, account).unwrap();
      assert_ne!(account_data.decay_snapshot, Decimal::zero());
   }

   #[test]
   fn test_execute_endgame() {
      let mut deps = mock_dependencies();
      let now = Timestamp::from_seconds(1_000_000);

      // Initialize DecayGame in storage
      let mut decay_game = DecayGame::new(now, now.plus_seconds(3600));
      decay_game.total = Uint128::new(1000);
      DECAY_GAME.save(&mut deps.storage, &decay_game).unwrap();

      // Execute endgame
      let result = execute_endgame(&mut deps.storage, now.plus_seconds(3601));
      assert!(result.is_ok());
      assert_eq!(result.unwrap(), Uint128::new(1000));
   }

   #[test]
   fn test_execute_donate() {
      let mut deps = mock_dependencies();
      let coin = Coin { denom: "utoken".to_string(), amount: Uint128::new(500) };

      // Execute donate
      let result = execute_donate(&mut deps.storage, coin.clone());
      assert!(result.is_ok());

      // Verify that the reward was updated in storage
      let stored_coin = REWARDS.load(&deps.storage, "utoken".to_string()).unwrap();
      assert_eq!(stored_coin.amount, Uint128::new(500));
   }

   #[test]
   fn test_execute_restart() {
      let mut deps = mock_dependencies();
      let now = Timestamp::from_seconds(1_000_000);
      let config = setup_config();

      // Initialize DecayGame and a mock account
      let mut decay_game = DecayGame::new(now, now.plus_seconds(3600));
      decay_game.total = Uint128::new(1000);
      decay_game.exited = Uint128::new(500); // Simulate players exited
      decay_game.rewards = Uint128::new(500);
      DECAY_GAME.save(&mut deps.storage, &decay_game).unwrap();

      // Execute restart error not ended
      execute_restart(&mut deps.storage, now, &config).unwrap_err();

      // Execute restart
      execute_restart(&mut deps.storage, now.plus_seconds(4000), &config).unwrap();

      // Verify that the new game has been created with reset state
      let new_game = DECAY_GAME.load(&deps.storage).unwrap();
      assert!(new_game.total.is_zero());
      assert!(new_game.exited.is_zero());
   }

   #[test]
   fn test_execute_post_swap() {
      let mut deps = mock_dependencies();
      let config = setup_config();
      let winner = Addr::unchecked("winner");
      let win_coin = Coin { denom: "win_token".to_string(), amount: Uint128::new(1000) };

      let coin = Coin { denom: "utoken".to_string(), amount: Uint128::new(1000) };

      // Add mock reward to the REWARDS map
      REWARDS.save(&mut deps.storage, "utoken".to_string(), &coin).unwrap();

      // Execute post_swap
      let response =
         execute_post_swap(&mut deps.storage, &config, winner.clone(), win_coin.clone()).unwrap();

      // Verify that response contains BankMsg::Send messages
      let bank_msgs: Vec<&BankMsg> = response
         .messages
         .iter()
         .filter_map(|msg| {
            if let cosmwasm_std::CosmosMsg::Bank(msg) = &msg.msg {
               Some(msg)
            } else {
               None
            }
         })
         .collect();

      assert!(bank_msgs.len() >= 2); // Should contain at least 2 send messages
   }
}
