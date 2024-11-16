use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Empty, Order, StdResult, Storage, Uint128};
use cw_storage_plus::Map;

use crate::normalize;

#[cw_serde]
pub struct RewardInfo {
   pub user: Addr,
   pub denom: String,
   /// Amount of rewards accrued
   pub accrued: Uint128,
}

impl RewardInfo {
   pub fn new(user: Addr, denom: String) -> Self {
      Self { user, denom, accrued: Uint128::zero() }
   }
}

pub struct RewardsSM<'a> {
   pub global_indices: Map<&'a str, Empty>,
   pub user_rewards: Map<(&'a Addr, &'a str), RewardInfo>,
}

impl<'a> RewardsSM<'a> {
   pub const fn new() -> Self {
      Self { global_indices: Map::new("rwd/gi"), user_rewards: Map::new("rwd/ur") }
   }

   /// Claim the accrued rewards for the specified user, setting the accrued rewards to zero.
   pub fn claim_accrued(&self, storage: &mut dyn Storage, user: &Addr) -> StdResult<Vec<Coin>> {
      let global_indices = self
         .global_indices
         .range(storage, None, None, Order::Ascending)
         .collect::<StdResult<Vec<_>>>()?;

      let mut accrued = Vec::with_capacity(global_indices.len());
      for (denom, _) in global_indices {
         let mut reward_info = self
            .user_rewards
            .may_load(storage, (user, &denom))?
            .unwrap_or_else(|| RewardInfo::new(user.clone(), denom.clone()));

         accrued.push(Coin::new(reward_info.accrued, &denom));

         reward_info.accrued = Uint128::zero();

         self.user_rewards.save(storage, (user, &denom), &reward_info)?;
      }

      Ok(normalize(accrued))
   }

   /// Get the list of accrued rewards for the specified user.
   pub fn get_accrued(&self, storage: &dyn Storage, user: &Addr) -> StdResult<Vec<Coin>> {
      let global_indices = self
         .global_indices
         .range(storage, None, None, Order::Ascending)
         .collect::<StdResult<Vec<_>>>()?;

      let mut accrued = Vec::with_capacity(global_indices.len());
      for (denom, _) in global_indices {
         let reward_info = self
            .user_rewards
            .may_load(storage, (user, &denom))?
            .unwrap_or_else(|| RewardInfo::new(user.clone(), denom.clone()));

         accrued.push(Coin::new(reward_info.accrued.u128(), &denom));
      }

      Ok(normalize(accrued))
   }

   /// Add to the accrued rewards for the specified user.
   pub fn add_accrued_rewards(
      &self,
      storage: &mut dyn Storage,
      user: &Addr,
      rewards: &Vec<Coin>,
   ) -> StdResult<()> {
      for coin in rewards {
         let mut reward_info = self
            .user_rewards
            .may_load(storage, (user, &coin.denom))?
            .unwrap_or_else(|| RewardInfo::new(user.clone(), coin.denom.clone()));

         reward_info.accrued += coin.amount;
         self.user_rewards.save(storage, (user, &coin.denom), &reward_info)?;

         // If the global index for this denom is not set, initialize it
         if !self.global_indices.has(storage, &coin.denom) {
            self.global_indices.save(storage, &coin.denom, &Empty {})?;
         }
      }

      Ok(())
   }
}

impl<'a> Default for RewardsSM<'a> {
   fn default() -> Self {
      Self::new()
   }
}

#[cfg(test)]
mod test {
   use cosmwasm_std::{coin, coins, testing::mock_dependencies, Addr};

   use super::RewardsSM;

   #[test]
   fn get_accrued_with_zero() {
      let mut odeps = mock_dependencies();
      let state = RewardsSM::new();
      let deps = odeps.as_mut();

      let user = Addr::unchecked("user");

      let ret = state.get_accrued(deps.storage, &user).expect("get works");
      assert!(ret.is_empty());
   }

   #[test]
   fn add_accrued_rewards() {
      let mut odeps = mock_dependencies();
      let state = RewardsSM::new();
      let deps = odeps.as_mut();

      let user = Addr::unchecked("user");
      state.add_accrued_rewards(deps.storage, &user, &coins(100u128, "ucoin")).expect("add works");
      let ret = state.get_accrued(deps.storage, &user).expect("get works");
      assert_eq!(ret.len(), 1);
      assert_eq!(ret[0].amount.u128(), 100u128);
      assert_eq!(ret[0].denom, "ucoin");
   }

   #[test]
   fn claim_accrued() {
      let mut odeps = mock_dependencies();
      let state = RewardsSM::new();
      let deps = odeps.as_mut();

      let user = Addr::unchecked("user");

      state.add_accrued_rewards(deps.storage, &user, &coins(100u128, "ucoin")).expect("add works");
      let ret = state.claim_accrued(deps.storage, &user).expect("claim works");
      assert_eq!(ret.len(), 1);
      assert_eq!(ret[0].amount.u128(), 100u128);
      assert_eq!(ret[0].denom, "ucoin");

      //check that after claim the accured rew are empty
      let ret = state.get_accrued(deps.storage, &user).expect("get works");
      assert!(ret.is_empty());
   }

   #[test]
   fn claim_with_zero_accrued() {
      let mut odeps = mock_dependencies();
      let state = RewardsSM::new();
      let deps = odeps.as_mut();

      let user = Addr::unchecked("user");

      state.claim_accrued(deps.storage, &user).expect("claim works");
      let ret = state.claim_accrued(deps.storage, &user).expect("claim works");
      assert!(ret.is_empty());
   }

   #[test]
   fn add_accrued_rewards_with_existing() {
      let mut odeps = mock_dependencies();
      let state = RewardsSM::new();
      let deps = odeps.as_mut();

      let user = Addr::unchecked("user");

      state.add_accrued_rewards(deps.storage, &user, &coins(100u128, "ucoin")).expect("add works");
      state.add_accrued_rewards(deps.storage, &user, &coins(100u128, "ucoin")).expect("add works");
      let ret = state.get_accrued(deps.storage, &user).expect("get works");
      assert_eq!(ret.len(), 1);
      assert_eq!(ret[0].amount.u128(), 200u128);
      assert_eq!(ret[0].denom, "ucoin");
   }

   #[test]
   fn add_accrued_rewards_multiple_coins() {
      let mut odeps = mock_dependencies();
      let state = RewardsSM::new();
      let deps = odeps.as_mut();

      let user = Addr::unchecked("user");
      state
         .add_accrued_rewards(
            deps.storage,
            &user,
            &vec![coin(100u128, "ucoin"), coin(200u128, "ucash")],
         )
         .expect("add works");
      let ret = state.get_accrued(deps.storage, &user).expect("get works");
      assert_eq!(ret.len(), 2);
      assert_eq!(ret[0].denom, "ucash");
      assert_eq!(ret[0].amount.u128(), 200u128);
      assert_eq!(ret[1].denom, "ucoin");
      assert_eq!(ret[1].amount.u128(), 100u128);
   }
}
