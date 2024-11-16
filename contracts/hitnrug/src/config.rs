use crate::msg::{Fees, InstantiateMsg, Points};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, Addr, Api, Decimal, StdResult, Storage, Uint128};
use cw_storage_plus::Item;

use crate::ContractError;

static CONFIG: Item<Config> = Item::new("config");

#[cw_serde]
pub struct Config {
   pub owner: Addr,
   pub ticket_denom: String,
   pub ticket_amount: Uint128,
   pub duration_seconds: u64,
   pub game_delay_sec: u64,
   pub delay_play_seconds: u64,
   pub winner_share: Decimal,
   pub fees: Fees,
   pub points: Points,
}

impl Config {
   pub fn new(msg: InstantiateMsg) -> Result<Self, ContractError> {
      let total_fee = msg.fees.fee_platform.bp + msg.fees.fee_ref.bp;
      ensure!(total_fee.lt(&Decimal::one()), ContractError::Invalid("fees_amounts".to_string()));
      let winner_share = Decimal::one() - total_fee;

      Ok(Self {
         owner: msg.owner,
         ticket_denom: msg.ticket_denom,
         ticket_amount: msg.ticket_amount,
         duration_seconds: msg.duration_seconds,
         game_delay_sec: msg.game_delay_sec,
         winner_share,
         delay_play_seconds: msg.delay_play_seconds,
         fees: msg.fees,
         points: msg.points,
      })
   }

   pub fn load(storage: &dyn Storage) -> StdResult<Self> {
      CONFIG.load(storage)
   }

   pub fn validate(&self, api: &dyn Api) -> Result<(), ContractError> {
      //address validations
      api.addr_validate(self.owner.as_str())?;
      api.addr_validate(self.fees.fee_platform.address.as_str())?;
      api.addr_validate(self.fees.fee_ref.address.as_str())?;

      ensure!(
         self.duration_seconds.gt(&0u64),
         ContractError::Invalid("duration_seconds".to_string())
      );

      ensure!(self.game_delay_sec.gt(&0u64), ContractError::Invalid("game_delay_sec".to_string()));

      ensure!(
         self.ticket_amount.gt(&Uint128::zero()),
         ContractError::Invalid("ticket_amount".to_string())
      );
      Ok(())
   }

   pub fn apply_update(&mut self, msg: ConfigUpdate) -> Result<(), ContractError> {
      if let Some(owner) = msg.owner {
         self.owner = owner;
      }

      if let Some(ticket_denom) = msg.ticket_denom {
         self.ticket_denom = ticket_denom;
      }

      if let Some(ticket_amount) = msg.ticket_amount {
         self.ticket_amount = ticket_amount;
      }

      if let Some(duration_seconds) = msg.duration_seconds {
         self.duration_seconds = duration_seconds;
      }

      if let Some(delay_play_seconds) = msg.delay_play_seconds {
         self.delay_play_seconds = delay_play_seconds;
      }

      if let Some(game_delay_sec) = msg.game_delay_sec {
         self.game_delay_sec = game_delay_sec;
      }

      if let Some(points) = msg.points {
         self.points = points;
      }

      if let Some(fees) = msg.fees {
         let total_fee = fees.fee_platform.bp + fees.fee_ref.bp;
         ensure!(total_fee.lt(&Decimal::one()), ContractError::Invalid("fees_amounts".to_string()));
         let winner_share = Decimal::one() - total_fee;
         self.fees = fees;
         self.winner_share = winner_share;
      }

      Ok(())
   }

   pub fn save(&self, storage: &mut dyn Storage) -> StdResult<()> {
      CONFIG.save(storage, self)
   }
}

#[cw_serde]
pub struct ConfigUpdate {
   pub owner: Option<Addr>,
   pub ticket_denom: Option<String>,
   pub ticket_amount: Option<Uint128>,
   pub duration_seconds: Option<u64>,
   pub delay_play_seconds: Option<u64>,
   pub game_delay_sec: Option<u64>,
   pub fees: Option<Fees>,
   pub points: Option<Points>,
}

#[cfg(test)]
mod tests {
   use crate::config::{Config, ConfigUpdate};
   use crate::msg::{Fee, Fees, InstantiateMsg, Point, Points};
   use crate::ContractError;
   use cosmwasm_std::testing::{mock_dependencies, MockStorage};
   use cosmwasm_std::{Addr, Decimal, Timestamp, Uint128};

   // Utility function to create a config from InstantiateMsg
   fn create_config(msg: InstantiateMsg) -> Result<Config, ContractError> {
      Config::new(msg)
   }

   // Test 1: Valid configuration creation
   #[test]
   fn test_valid_config_creation() {
      let msg = InstantiateMsg {
         owner: Addr::unchecked("owner"),
         ticket_denom: "token".to_string(),
         ticket_amount: Uint128::new(100),
         duration_seconds: 3600,
         game_delay_sec: 10,
         delay_play_seconds: 5,
         fees: Fees {
            fee_platform: Fee { bp: Decimal::percent(1), address: Addr::unchecked("platform_fee") },
            fee_ref: Fee { bp: Decimal::percent(2), address: Addr::unchecked("ref_fee") },
         },
         points: Points { keep: 6i64, hit: -4i64, help: Point { myself: 6i64, other: 4i64 } },
         starts_at: Timestamp::from_seconds(10000),
      };

      // Create the config using the given InstantiateMsg
      let config = create_config(msg).unwrap();

      // Check the values of the created config
      assert_eq!(config.owner, Addr::unchecked("owner"));
      assert_eq!(config.ticket_denom, "token");
      assert_eq!(config.ticket_amount, Uint128::new(100));
      assert_eq!(config.duration_seconds, 3600);
      assert_eq!(config.game_delay_sec, 10);
      assert_eq!(config.delay_play_seconds, 5);
      assert_eq!(config.fees.fee_platform.bp, Decimal::percent(1));
      assert_eq!(config.fees.fee_ref.bp, Decimal::percent(2));
      assert_eq!(config.points.keep, 6i64);
      assert_eq!(config.points.hit, -4i64);
      assert_eq!(config.points.help.myself, 6i64);
      assert_eq!(config.points.help.other, 4i64);
   }

   // Test 2: Invalid total fee validation (more than 100%)
   #[test]
   fn test_invalid_fee_total() {
      let msg = InstantiateMsg {
         owner: Addr::unchecked("owner"),
         ticket_denom: "token".to_string(),
         ticket_amount: Uint128::new(100),
         duration_seconds: 3600,
         game_delay_sec: 10,
         delay_play_seconds: 5,
         fees: Fees {
            fee_platform: Fee {
               bp: Decimal::percent(90),
               address: Addr::unchecked("platform_fee"),
            },
            fee_ref: Fee { bp: Decimal::percent(15), address: Addr::unchecked("ref_fee") },
         },
         points: Points { keep: 6i64, hit: -4i64, help: Point { myself: 6i64, other: 4i64 } },
         starts_at: Timestamp::from_seconds(10000),
      };

      // Ensure the total fee is invalid and throws an error
      let result = create_config(msg);
      assert!(result.is_err(), "Config creation should fail due to invalid fee total");
   }

   // Test 3: Invalid address validation (e.g., invalid owner address)
   #[test]
   fn test_invalid_address_validation() {
      let mut odeps = mock_dependencies();
      let deps = odeps.as_mut();
      let msg = InstantiateMsg {
         owner: Addr::unchecked("invalid_owner"),
         ticket_denom: "token".to_string(),
         ticket_amount: Uint128::new(100),
         duration_seconds: 3600,
         game_delay_sec: 10,
         delay_play_seconds: 5,
         fees: Fees {
            fee_platform: Fee { bp: Decimal::percent(1), address: Addr::unchecked("platform_fee") },
            fee_ref: Fee { bp: Decimal::percent(2), address: Addr::unchecked("ref_fee") },
         },
         points: Points { keep: 6i64, hit: -4i64, help: Point { myself: 6i64, other: 4i64 } },
         starts_at: Timestamp::from_seconds(10000),
      };

      // Try creating config and expect validation failure for invalid address
      let config = create_config(msg).unwrap();
      config.validate(deps.api).unwrap_err();
   }

   // Test 4: Config update application
   #[test]
   fn test_apply_config_update() {
      let mut config = Config {
         owner: Addr::unchecked("owner"),
         ticket_denom: "token".to_string(),
         ticket_amount: Uint128::new(100),
         duration_seconds: 3600,
         game_delay_sec: 10,
         delay_play_seconds: 5,
         fees: Fees {
            fee_platform: Fee { bp: Decimal::percent(1), address: Addr::unchecked("platform_fee") },
            fee_ref: Fee { bp: Decimal::percent(2), address: Addr::unchecked("ref_fee") },
         },
         points: Points { keep: 6i64, hit: -4i64, help: Point { myself: 6i64, other: 4i64 } },
         winner_share: Decimal::percent(97),
      };

      let update_msg = ConfigUpdate {
         owner: Some(Addr::unchecked("new_owner")),
         ticket_denom: Some("new_token".to_string()),
         ticket_amount: Some(Uint128::new(200)),
         duration_seconds: Some(7200),
         delay_play_seconds: Some(10),
         game_delay_sec: Some(15),
         points: Some(Points {
            keep: 10i64,
            hit: -5i64,
            help: Point { myself: 10i64, other: 5i64 },
         }),
         fees: Some(Fees {
            fee_platform: Fee {
               bp: Decimal::percent(2),
               address: Addr::unchecked("new_platform_fee"),
            },
            fee_ref: Fee { bp: Decimal::percent(3), address: Addr::unchecked("new_ref_fee") },
         }),
      };

      config.apply_update(update_msg).expect("Failed to apply update");

      // Validate updated values
      assert_eq!(config.owner, Addr::unchecked("new_owner"));
      assert_eq!(config.ticket_denom, "new_token");
      assert_eq!(config.ticket_amount, Uint128::new(200));
      assert_eq!(config.duration_seconds, 7200);
      assert_eq!(config.fees.fee_platform.bp, Decimal::percent(2));
      assert_eq!(config.fees.fee_ref.bp, Decimal::percent(3));
      assert_eq!(config.winner_share, Decimal::percent(95));
      assert_eq!(config.points.keep, 10i64);
      assert_eq!(config.points.hit, -5i64);
      assert_eq!(config.points.help.myself, 10i64);
      assert_eq!(config.points.help.other, 5i64);
   }

   // Test 5: Save and load config from storage
   #[test]
   fn test_save_and_load_config() {
      let msg = InstantiateMsg {
         owner: Addr::unchecked("owner"),
         ticket_denom: "token".to_string(),
         ticket_amount: Uint128::new(100),
         duration_seconds: 3600,
         game_delay_sec: 10,
         delay_play_seconds: 5,
         fees: Fees {
            fee_platform: Fee { bp: Decimal::percent(1), address: Addr::unchecked("platform_fee") },
            fee_ref: Fee { bp: Decimal::percent(2), address: Addr::unchecked("ref_fee") },
         },
         points: Points { keep: 6i64, hit: -4i64, help: Point { myself: 6i64, other: 4i64 } },
         starts_at: Timestamp::from_seconds(10000),
      };

      let config = create_config(msg).unwrap();

      // Simulate saving to storage
      let mut storage = MockStorage::default();
      config.save(&mut storage).unwrap();

      // Simulate loading from storage
      let loaded_config = Config::load(&storage).expect("Failed to load config");

      // Verify that the saved and loaded config are the same
      assert_eq!(config, loaded_config);
   }
}
