use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Timestamp, Uint128};
use std::ops::Sub;
use thiserror::Error;

// Core Decay Game
#[cw_serde]
#[derive(Default)]
pub struct DecayGame {
   pub decay_starts_at: Timestamp,
   pub decay_ends_at: Timestamp,
   pub total: Uint128,
   pub exited: Uint128,
   pub rewards: Uint128,
}

impl DecayGame {
   pub const fn new(decay_starts_at: Timestamp, decay_ends_at: Timestamp) -> Self {
      DecayGame {
         decay_starts_at,
         decay_ends_at,
         total: Uint128::zero(),
         exited: Uint128::zero(),
         rewards: Uint128::zero(),
      }
   }

   pub fn validate(&self, now: &Timestamp) -> Result<(), DecayGameError> {
      if self.decay_starts_at.lt(now) {
         return Err(DecayGameError::Invalid("decay_starts_at".to_string()));
      }
      if self.decay_ends_at.lt(&self.decay_starts_at) {
         return Err(DecayGameError::Invalid("decay_ends_at".to_string()));
      }

      Ok(())
   }

   pub fn join(
      &mut self,
      amount: Uint128,
      now: &Timestamp,
   ) -> Result<DecayGameAccount, DecayGameError> {
      if now.gt(&self.decay_ends_at) {
         return Err(DecayGameError::Invalid("game_ended".to_string()));
      }
      self.total += amount;

      Ok(DecayGameAccount { amount, decay_snapshot: Decimal::zero(), pending: Uint128::zero() })
   }

   pub fn exit(&mut self, now: &Timestamp, account: &mut DecayGameAccount) -> () {
      let factor = decay_factor(&self, now);
      let pending = account.amount.mul_floor(factor);

      self.exited += pending;

      account.pending = pending;

      account.decay_snapshot = factor;
   }

   pub fn pending_claims(&self, account: &DecayGameAccount) -> Uint128 {
      account.pending
   }

   pub fn pending_rewards(&self) -> Uint128 {
      self.total - self.exited
   }

   pub fn claim(&mut self, account: &mut DecayGameAccount) -> Uint128 {
      let amount = self.pending_claims(account);
      account.pending = Uint128::zero();
      amount
   }

   pub fn distribute_rewards(&mut self, now: &Timestamp) -> Result<Uint128, DecayGameError> {
      if now.le(&self.decay_ends_at) {
         return Err(DecayGameError::DecayNotEnded {});
      }
      match self.total - self.exited - self.rewards == Uint128::zero() {
         true => {
            return Err(DecayGameError::NoRewards {});
         }

         false => {
            self.rewards = self.total - self.exited;
            return Ok(self.rewards);
         }
      }
   }
}

#[cw_serde]
#[derive(Default)]
pub struct DecayGameAccount {
   pub amount: Uint128,
   pub decay_snapshot: Decimal,
   pub pending: Uint128,
}

#[derive(Error, Debug)]
pub enum DecayGameError {
   #[error("DecayNotEnded")]
   DecayNotEnded {},

   #[error("NoRewards")]
   NoRewards {},

   #[error("Invalid: {0}")]
   Invalid(String),
}

/// The amount of decay remaining in a linear model
fn decay_factor(equilibria_pool: &DecayGame, now: &Timestamp) -> Decimal {
   if now.le(&equilibria_pool.decay_starts_at) {
      return Decimal::one();
   }
   if now.gt(&equilibria_pool.decay_ends_at) {
      return Decimal::zero();
   }
   let remaning = equilibria_pool.decay_ends_at.seconds().sub(now.seconds());
   let duration =
      equilibria_pool.decay_ends_at.seconds().sub(equilibria_pool.decay_starts_at.seconds());

   Decimal::from_ratio(remaning, duration)
}

#[cfg(test)]
mod tests {

   use super::*;

   #[test]
   fn test_validate() {
      let now = Timestamp::from_seconds(1000);
      let mut decay_starts_at = Timestamp::from_seconds(100);
      let mut decay_ends_at = Timestamp::from_seconds(1000);

      // now > start => Error
      let pool = DecayGame::new(decay_starts_at, decay_ends_at);
      pool.validate(&now).unwrap_err();

      // sart > end => Error
      decay_starts_at = Timestamp::from_seconds(1001);
      let pool = DecayGame::new(decay_starts_at, decay_ends_at);
      pool.validate(&now).unwrap_err();

      // now < sart < end => Success
      decay_ends_at = Timestamp::from_seconds(1002);
      let pool = DecayGame::new(decay_starts_at, decay_ends_at);
      pool.validate(&now).unwrap();
   }

   #[test]
   fn lifecycle() {
      let mut now = Timestamp::from_seconds(1);
      let decay_starts_at = Timestamp::from_seconds(100);
      let decay_ends_at = Timestamp::from_seconds(1000);

      let mut pool = DecayGame::new(decay_starts_at, decay_ends_at);
      pool.validate(&now).unwrap();

      let mut account = pool.join(Uint128::from(100u128), &now).unwrap();
      assert_eq!(pool.total, Uint128::from(100u128));
      assert_eq!(account.amount, Uint128::from(100u128));
      assert_eq!(account.pending, Uint128::zero());
      assert_eq!(account.decay_snapshot, Decimal::zero());

      //No decay now is still before the start date
      pool.exit(&now, &mut account);
      assert_eq!(pool.exited, Uint128::from(100u128));
      assert_eq!(pool.pending_rewards(), Uint128::zero());
      assert_eq!(account.amount, Uint128::from(100u128));
      assert_eq!(account.pending, Uint128::from(100u128));
      assert_eq!(account.decay_snapshot, Decimal::one());
      assert_eq!(pool.pending_claims(&account), Uint128::from(100u128));

      let claim = pool.claim(&mut account);
      assert_eq!(claim, Uint128::from(100u128));
      assert_eq!(account.pending, Uint128::zero());
      assert_eq!(pool.pending_claims(&account), Uint128::zero());

      let mut account = pool.join(Uint128::from(100u128), &now).unwrap();
      assert_eq!(pool.total, Uint128::from(200u128));

      now = Timestamp::from_seconds(550);
      pool.exit(&now, &mut account);
      assert_eq!(account.amount, Uint128::from(100u128));
      assert_eq!(account.pending, Uint128::from(50u128));
      assert_eq!(account.decay_snapshot, Decimal::from_ratio(Uint128::one(), Uint128::from(2u128)));

      let claim = pool.claim(&mut account);
      assert_eq!(claim, Uint128::from(50u128));
      assert_eq!(account.pending, Uint128::zero());
      assert_eq!(pool.pending_claims(&account), Uint128::zero());

      assert_eq!(pool.pending_rewards(), Uint128::from(50u128));
      pool.distribute_rewards(&now).unwrap_err();

      // join till the end
      let mut account = pool.join(Uint128::from(100u128), &now).unwrap();

      //move now after ends_at
      now = Timestamp::from_seconds(2000);

      // exit after end => decay 0
      pool.exit(&now, &mut account);
      assert_eq!(account.amount, Uint128::from(100u128));
      assert_eq!(account.pending, Uint128::zero());
      assert_eq!(account.decay_snapshot, Decimal::zero());

      //distribute the rewards
      let rewards = pool.distribute_rewards(&now).unwrap();
      assert_eq!(rewards, Uint128::from(150u128));

      pool.distribute_rewards(&now).unwrap_err();

      //join after end => err
      pool.join(Uint128::from(100u128), &now).unwrap_err();
   }
}
