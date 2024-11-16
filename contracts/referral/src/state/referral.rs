use cosmwasm_std::{Order, StdError, StdResult, Storage};
use cw_storage_plus::Map;

pub struct ReferralSM<'a> {
   pub user_to_code: Map<&'a str, String>,
   pub code_to_user: Map<&'a str, String>,
   pub referee_to_user: Map<&'a str, String>,
}

impl<'a> ReferralSM<'a> {
   pub const fn new() -> Self {
      Self {
         user_to_code: Map::new("ref/utc"),
         code_to_user: Map::new("ref/ctu"),
         referee_to_user: Map::new("ref/rft"),
      }
   }

   pub fn gen_code(
      &self,
      storage: &mut dyn Storage,
      user: &String,
      code: &String,
   ) -> StdResult<()> {
      // Check if the user already has a code
      let existing_code = self.user_to_code.may_load(storage, user.as_str())?;
      if existing_code.is_some() {
         return Err(StdError::generic_err("User already has a code"));
      }

      // Save the new code
      self.user_to_code.save(storage, user.as_str(), code)?;
      self.code_to_user.save(storage, code.as_str(), user)?;
      Ok(())
   }

   pub fn get_code(&self, storage: &dyn Storage, user: &String) -> StdResult<String> {
      Ok(self.user_to_code.may_load(storage, user.as_str())?.unwrap_or_default())
   }

   pub fn get_addr(&self, storage: &dyn Storage, code: &String) -> StdResult<String> {
      // Retrieve the code for the given user
      match self.code_to_user.load(storage, &code) {
         Ok(addr) => Ok(addr),
         Err(_) => Err(StdError::not_found("Addr")),
      }
   }

   pub fn add_referee(
      &self,
      storage: &mut dyn Storage,
      referee: &String,
      code: &String,
   ) -> StdResult<()> {
      let existing_code: Option<String> = self.code_to_user.may_load(storage, code)?;
      if existing_code.is_none() {
         return Err(StdError::generic_err("Code Not Found"));
      }

      // Check if the referee is already added
      let existing_user = self.referee_to_user.may_load(storage, referee.as_str())?;
      if existing_user.is_some() {
         return Err(StdError::generic_err("Referee already added"));
      }

      // Save the referee
      self.referee_to_user.save(storage, referee.as_str(), &existing_code.unwrap())?;
      Ok(())
   }

   pub fn get_referrer(&self, storage: &dyn Storage, referee: &String) -> StdResult<String> {
      // Retrieve the user who referred the given referee
      Ok(self.referee_to_user.may_load(storage, referee.as_str())?.unwrap_or_default())
   }

   pub fn get_referral_struct(&self, storage: &dyn Storage, user: &str) -> StdResult<Vec<String>> {
      let mut referees = Vec::new();

      let entries = self
         .referee_to_user
         .range(storage, None, None, Order::Ascending)
         .collect::<StdResult<Vec<_>>>()?;

      for entry in entries {
         let (referee, stored_user) = entry;
         if stored_user == user {
            referees.push(referee.to_string());
         }
      }

      Ok(referees)
   }
}

#[cfg(test)]
mod test {
   use super::ReferralSM;
   use cosmwasm_std::testing::mock_dependencies;

   #[test]
   fn test_gen_code() {
      let mut deps = mock_dependencies();
      let referral_system = ReferralSM::new();

      let user = "user1".to_string();
      let code = "code123".to_string();

      // Generate a new code
      referral_system.gen_code(&mut deps.storage, &user, &code).unwrap();

      // Attempt to generate a code for the same user should fail
      let result = referral_system.gen_code(&mut deps.storage, &user, &"new_code".to_string());
      assert!(result.is_err());

      // Verify the code is correct
      let stored_code = referral_system.get_code(&deps.storage, &user).unwrap();
      assert_eq!(stored_code, code);
   }

   #[test]
   fn test_add_referee() {
      let mut deps = mock_dependencies();
      let referral_system = ReferralSM::new();

      let user = "user1".to_string();
      let code = "code".to_string();
      let referee = "referee1".to_string();

      // Generate a new code
      referral_system.gen_code(&mut deps.storage, &user, &code).unwrap();

      // Add a new referee
      referral_system.add_referee(&mut deps.storage, &referee, &code).unwrap();

      // Attempt to add the same referee should fail
      let result = referral_system.add_referee(&mut deps.storage, &referee, &user);
      assert!(result.is_err());

      // Verify the referrer is correct
      let stored_user = referral_system.get_referrer(&deps.storage, &referee).unwrap();
      assert_eq!(stored_user, user);
   }

   #[test]
   fn test_get_referral_struct() {
      let mut deps = mock_dependencies();
      let referral_system = ReferralSM::new();

      let user = "user1".to_string();
      let code = "code".to_string();
      let referee1 = "referee1".to_string();
      let referee2 = "referee2".to_string();

      // Generate a new code
      referral_system.gen_code(&mut deps.storage, &user, &code).unwrap();

      // Add referees
      referral_system.add_referee(&mut deps.storage, &referee1, &code).unwrap();
      referral_system.add_referee(&mut deps.storage, &referee2, &code).unwrap();

      // Get all referees for the user
      let referees = referral_system.get_referral_struct(&deps.storage, &user).unwrap();
      assert_eq!(referees, vec![referee1, referee2]);
   }
}
