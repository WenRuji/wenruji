use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
   ensure, to_json_binary, Addr, Binary, Decimal, StdResult, Storage, Timestamp, Uint128,
};
use cw_storage_plus::{Item, Map};
use wenruji_rs::{DecayGame, DecayGameAccount};

use crate::{config::Config, msg::PlayMsg, ContractError};
#[cw_serde]
pub struct GameBase {
   pub decay_game: DecayGame,
   pub current_winner: Option<(Addr, i64)>,
}

impl GameBase {
   pub fn new(decay_starts_at: Timestamp, decay_ends_at: Timestamp) -> Self {
      Self { decay_game: DecayGame::new(decay_starts_at, decay_ends_at), current_winner: None }
   }
}

#[cw_serde]
pub struct GameSmSnapshot {
   pub decay_game: DecayGame,
   pub accounts: Vec<(Addr, DecayGameAccount)>,
   pub current_winner: Option<(Addr, i64)>,
   pub players: Vec<PlayerStatus>,
   pub referrals: Vec<(Addr, Decimal)>,
}

#[cw_serde]

pub struct PlayerStatus {
   address: Addr,
   points: i64,
   history: Vec<PlayMsg>,
   last_play: Timestamp,
}

impl PlayerStatus {
   pub fn new(account: Addr) -> Self {
      Self { address: account, points: 0i64, history: vec![], last_play: Timestamp::default() }
   }
}

pub struct GameSM<'a> {
   pub game_base: Item<GameBase>,
   pub accounts: Map<&'a Addr, DecayGameAccount>,
   pub ref_weight: Map<&'a Addr, Decimal>,
   pub players: Map<&'a Addr, PlayerStatus>,
}

impl<'a> GameSM<'a> {
   pub const fn new() -> Self {
      Self {
         game_base: Item::new("gm/b"),
         accounts: Map::new("gm/a"),
         ref_weight: Map::new("gm/rf"),
         players: Map::new("gm/p"),
      }
   }

   pub fn initialize(
      &self,
      storage: &mut dyn Storage,
      decay_starts_at: Timestamp,
      decay_ends_at: Timestamp,
   ) -> StdResult<()> {
      let game_base = GameBase::new(decay_starts_at, decay_ends_at);
      self.game_base.save(storage, &game_base)?;
      Ok(())
   }

   pub fn increase_ref(&self, storage: &mut dyn Storage, addr: &Addr) -> Result<(), ContractError> {
      self.ref_weight.update(storage, addr, |weight| -> StdResult<_> {
         Ok(weight.unwrap_or(Decimal::zero()) + Decimal::one())
      })?;
      Ok(())
   }

   pub fn join(
      &self,
      storage: &mut dyn Storage,
      now: Timestamp,
      account: &Addr,
      amount: Uint128,
   ) -> Result<(), ContractError> {
      let mut game_base = self.game_base.load(storage)?;
      let account_data = game_base.decay_game.join(amount, &now)?;
      self.check_winner(&mut game_base, account, 0i64);
      self.game_base.save(storage, &game_base)?;
      self.accounts.save(storage, account, &account_data)?;
      self.players.save(storage, &account, &PlayerStatus::new(account.clone()))?;
      Ok(())
   }

   pub fn exit(
      &self,
      storage: &mut dyn Storage,
      now: Timestamp,
      account: &Addr,
   ) -> Result<(Uint128, Decimal), ContractError> {
      let mut game_base = self.game_base.load(storage)?;
      ensure!(
         now.lt(&game_base.decay_game.decay_ends_at),
         ContractError::Invalid("game_ended".to_string())
      );

      let mut account_data = self.accounts.load(storage, account)?;
      if !account_data.decay_snapshot.is_zero() {
         return Err(ContractError::Invalid("already_exited".to_string()));
      }

      game_base.decay_game.exit(&now, &mut account_data);
      let amount = game_base.decay_game.claim(&mut account_data);
      self.game_base.save(storage, &game_base)?;
      self.accounts.save(storage, account, &account_data)?;
      Ok((amount, account_data.decay_snapshot))
   }

   pub fn endgame(
      self,
      storage: &mut dyn Storage,
      now: Timestamp,
   ) -> Result<(Option<(Addr, i64)>, Uint128), ContractError> {
      let mut game_base = self.game_base.load(storage)?;
      let amount = game_base.decay_game.distribute_rewards(&now)?;
      self.game_base.save(storage, &game_base)?;
      let winner = game_base.current_winner;
      Ok((winner, amount))
   }

   pub fn restart(
      &self,
      storage: &mut dyn Storage,
      decay_starts_at: Timestamp,
      decay_ends_at: Timestamp,
   ) -> Result<(), ContractError> {
      self.accounts.clear(storage);
      self.ref_weight.clear(storage);
      self.players.clear(storage);
      self.initialize(storage, decay_starts_at, decay_ends_at)?;

      Ok(())
   }

   pub fn check_winner(&self, game_base: &mut GameBase, account: &Addr, points: i64) {
      if game_base.current_winner.is_none() || points > game_base.current_winner.clone().unwrap().1
      {
         game_base.current_winner = Some((account.clone(), points));
      }
   }

   pub fn update_winner(
      &self,
      storage: &dyn Storage,
      game_base: &mut GameBase,
   ) -> Result<(), ContractError> {
      let mut max_points = i64::MIN;
      let mut winner: Option<Addr> = None;
      for item in self.players.range(storage, None, None, cosmwasm_std::Order::Ascending) {
         let (addr, player) = item?;
         if player.points > max_points {
            max_points = player.points;
            winner = Some(addr.clone());
         }
      }
      game_base.current_winner = winner.map(|addr| (addr, max_points));
      Ok(())
   }

   fn apply_points(&self, player: &mut PlayerStatus, points: i64) {
      player.points = (player.points.saturating_add(points)).max(0);
   }

   pub fn update_play(
      &self,
      player: &mut PlayerStatus,
      points: i64,
      msg: PlayMsg,
      now: Timestamp,
      delay: u64,
   ) -> Result<(), ContractError> {
      ensure!(
         now.ge(&player.last_play.plus_seconds(delay)),
         ContractError::Invalid("play_timestamp".to_string())
      );
      self.apply_points(player, points);
      player.history.push(msg);
      player.last_play = now;
      Ok(())
   }

   pub fn play(
      &self,
      storage: &mut dyn Storage,
      msg: PlayMsg,
      account: &Addr,
      config: &Config,
      now: Timestamp,
   ) -> Result<(), ContractError> {
      let mut game_base = self.game_base.load(storage)?;
      let mut player = self.players.load(storage, account)?;

      // Define points and target player update based on `PlayMsg`
      let (player_points, target_points, target, hit) = match &msg {
         PlayMsg::Keep {} => (config.points.keep, 0, None, false),
         PlayMsg::Hit { target } => (0, config.points.hit, Some(target), true),
         PlayMsg::Help { target } => {
            (config.points.help.myself, config.points.help.other, Some(target), false)
         }
      };

      if let Some(target) = target {
         ensure!(target != account, ContractError::Invalid("target".to_string()));
      }

      // Update the player's points and last play time
      self.update_play(&mut player, player_points, msg.clone(), now, config.delay_play_seconds)?;

      // Apply points to target player if applicable
      if let Some(target) = target {
         let mut target_player = self.players.load(storage, target)?;
         self.apply_points(&mut target_player, target_points);
         self.players.save(storage, target, &target_player)?;
         if hit && game_base.current_winner.clone().unwrap().0 == target {
            self.update_winner(storage, &mut game_base)?;
         } else {
            self.check_winner(&mut game_base, &target, target_player.points);
         }
      }

      // Save updated player state and check winner
      self.players.save(storage, account, &player)?;
      self.check_winner(&mut game_base, &account, player.points);
      self.game_base.save(storage, &game_base)?;

      Ok(())
   }

   pub fn is_ended(
      &self,
      storage: &mut dyn Storage,
      now: Timestamp,
   ) -> Result<bool, ContractError> {
      let game_base = self.game_base.load(storage)?;
      if now.le(&game_base.decay_game.decay_ends_at) {
         return Ok(false);
      } else {
         return Ok(true);
      }
   }

   pub fn is_started(
      &self,
      storage: &mut dyn Storage,
      now: Timestamp,
   ) -> Result<bool, ContractError> {
      let game_base = self.game_base.load(storage)?;
      if now.lt(&game_base.decay_game.decay_starts_at) {
         return Ok(false);
      } else {
         return Ok(true);
      }
   }

   pub fn is_completed(
      &self,
      storage: &mut dyn Storage,
      now: Timestamp,
   ) -> Result<bool, ContractError> {
      let game_base = self.game_base.load(storage)?;
      if now.le(&game_base.decay_game.decay_ends_at)
         || !(game_base.decay_game.rewards
            == game_base.decay_game.total - game_base.decay_game.exited)
      {
         return Ok(false);
      } else {
         return Ok(true);
      }
   }

   pub fn has_joined(&self, storage: &mut dyn Storage, addr: &Addr) -> StdResult<bool> {
      Ok(self.accounts.has(storage, addr))
   }

   pub fn has_exited(&self, storage: &mut dyn Storage, addr: &Addr) -> StdResult<bool> {
      let acccount = self.accounts.load(storage, &addr)?;
      if !acccount.decay_snapshot.is_zero() {
         return Ok(true);
      }
      Ok(false)
   }

   pub fn get_ref_weights(
      &self,
      storage: &dyn Storage,
   ) -> Result<Vec<(Addr, Decimal)>, ContractError> {
      let referrals = self
         .ref_weight
         .range(storage, None, None, cosmwasm_std::Order::Ascending)
         .map(|item| {
            let (address, weight) = item?;
            Ok((address.clone(), weight))
         })
         .collect::<StdResult<Vec<_>>>()?;

      Ok(referrals)
   }

   pub fn get_snap(&self, storage: &dyn Storage) -> Result<Binary, ContractError> {
      let game_base = self.game_base.load(storage)?;
      let decay_game = game_base.decay_game.clone();
      let current_winner = game_base.current_winner.clone();

      let referrals = self.get_ref_weights(storage)?;

      let players = self
         .players
         .range(storage, None, None, cosmwasm_std::Order::Ascending)
         .map(|item| {
            let (_, player) = item?;
            Ok(player)
         })
         .collect::<StdResult<Vec<_>>>()?;

      let accounts: Vec<(Addr, DecayGameAccount)> = self
         .accounts
         .range(storage, None, None, cosmwasm_std::Order::Ascending)
         .map(|item| {
            let (address, account) = item?;
            Ok((address, account))
         })
         .collect::<StdResult<Vec<_>>>()?;

      let status = GameSmSnapshot { decay_game, current_winner, referrals, players, accounts };

      Ok(to_json_binary(&status)?)
   }
}

#[cfg(test)]
mod test {
   use super::*;
   use cosmwasm_std::testing::mock_dependencies;
   use cosmwasm_std::{from_json, Addr, Decimal, Timestamp, Uint128};

   #[test]
   fn test_initialize() {
      let mut odeps = mock_dependencies();
      let state = GameSM::new();
      let deps = odeps.as_mut();

      let start_time = Timestamp::from_seconds(0);
      let end_time = Timestamp::from_seconds(100);

      let result = state.initialize(deps.storage, start_time, end_time);
      assert!(result.is_ok(), "Initialization should succeed");

      // Boundary check: Verify decay start and end timestamps are set correctly
      let game_base = state.game_base.load(deps.storage).unwrap();
      assert_eq!(game_base.decay_game.decay_starts_at, start_time);
      assert_eq!(game_base.decay_game.decay_ends_at, end_time);
      assert!(game_base.current_winner.is_none());
   }

   #[test]
   fn test_increase_ref() {
      let mut odeps = mock_dependencies();
      let state = GameSM::new();
      let deps = odeps.as_mut();

      let user = Addr::unchecked("user");
      let user2 = Addr::unchecked("user2");

      state.increase_ref(deps.storage, &user).unwrap();
      state.increase_ref(deps.storage, &user2).unwrap();

      // Boundary check: Increase ref multiple times for same address
      state.increase_ref(deps.storage, &user).unwrap();
      let referrals = state.get_ref_weights(deps.storage).unwrap();

      assert_eq!(referrals.len(), 2, "There should be two unique referrers.");
      assert_eq!(referrals[0].1, Decimal::percent(200), "User1 should have two ref counts");
      assert_eq!(referrals[1].1, Decimal::percent(100), "User2 should have one ref count");
   }

   #[test]
   fn test_exit_with_already_exited_account() {
      let mut odeps = mock_dependencies();
      let state = GameSM::new();
      let deps = odeps.as_mut();

      let user = Addr::unchecked("user");

      state
         .initialize(deps.storage, Timestamp::from_seconds(0), Timestamp::from_seconds(100))
         .unwrap();

      state.join(deps.storage, Timestamp::from_seconds(1), &user, Uint128::new(100)).unwrap();

      // Simulate an exit
      let _ = state.exit(deps.storage, Timestamp::from_seconds(50), &user).unwrap();

      // Attempt a second exit, which should fail
      let second_exit = state.exit(deps.storage, Timestamp::from_seconds(50), &user);
      assert!(second_exit.is_err(), "Second exit should fail as already exited");
      assert_eq!(second_exit.unwrap_err().to_string(), "Invalid: already_exited");
   }

   #[test]
   fn test_endgame_with_no_participants() {
      let mut odeps = mock_dependencies();
      let state = GameSM::new();
      let deps = odeps.as_mut();

      state
         .initialize(deps.storage, Timestamp::from_seconds(0), Timestamp::from_seconds(100))
         .unwrap();

      // Endgame should succeed without participants and no winner should be set
      let result = state.endgame(deps.storage, Timestamp::from_seconds(101));
      assert_eq!(
         result.unwrap_err().to_string(),
         "NoRewards",
         "Endgame should throw no rewards error"
      );
   }

   #[test]
   fn test_apply_points_overflow() {
      let mut player = PlayerStatus {
         address: Addr::unchecked("user"),
         points: i64::MAX - 1,
         history: vec![],
         last_play: Timestamp::from_seconds(0),
      };
      let state = GameSM::new();

      state.apply_points(&mut player, 2);
      assert_eq!(player.points, i64::MAX, "Points should max out at i64::MAX");
   }

   #[test]
   fn test_has_ended_just_before_end() {
      let mut odeps = mock_dependencies();
      let state = GameSM::new();
      let deps = odeps.as_mut();

      state
         .initialize(deps.storage, Timestamp::from_seconds(0), Timestamp::from_seconds(100))
         .unwrap();

      // Boundary test: Game should not end right before decay_ends_at
      let result = state.is_ended(deps.storage, Timestamp::from_seconds(99));
      assert!(result.is_ok());
      assert!(!result.unwrap(), "Game should not be ended just before end time");
   }

   #[test]
   fn test_get_snap_structure() {
      let mut odeps = mock_dependencies();
      let state = GameSM::new();
      let deps = odeps.as_mut();

      let start_time = Timestamp::from_seconds(0);
      let end_time = Timestamp::from_seconds(100);

      // Initialize game state
      state.initialize(deps.storage, start_time, end_time).unwrap();

      // Add a player and increase ref counts
      let user = Addr::unchecked("user");
      let join_amount = Uint128::new(100);
      state.join(deps.storage, Timestamp::from_seconds(1), &user, join_amount).unwrap();
      state.increase_ref(deps.storage, &user).unwrap();

      // Take a snapshot
      let snap = state.get_snap(deps.storage).unwrap();
      let snapshot: GameSmSnapshot = from_json(snap).unwrap();
      println!("{:?}", snapshot);

      // Verify snapshot structure
      assert_eq!(
         snapshot.decay_game.decay_starts_at, start_time,
         "Decay start should match initialized time"
      );
      assert_eq!(
         snapshot.decay_game.decay_ends_at, end_time,
         "Decay end should match initialized time"
      );
      assert!(
         snapshot.current_winner.is_some(),
         "Winner by definition is the first who joins if no points are made"
      );

      // Check players and referral counts in snapshot
      assert_eq!(snapshot.players.len(), 1, "There should be one player in snapshot");
      assert_eq!(snapshot.referrals.len(), 1, "There should be one referral in snapshot");
      assert_eq!(snapshot.referrals[0].1, Decimal::percent(100), "Referral weight should match");
   }
}
