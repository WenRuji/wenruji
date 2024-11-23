use crate::msg::{Contracts, Fees, InstantiateMsg};
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
   pub game_delay: u64,
   pub contracts: Contracts,
   pub fees: Vec<(Addr, Decimal)>,
   pub winner_share: Decimal,
   pub donation_addrs: Vec<Addr>,
}

impl Config {
   pub fn new(msg: InstantiateMsg) -> Result<Self, ContractError> {
      let total_fee = msg.fees.fee_platform.fee + msg.fees.fee_nami.fee + msg.fees.fee_ref.fee;
      ensure!(total_fee.lt(&Decimal::one()), ContractError::Invalid("fees_amounts".to_string()));
      let winner_share = Decimal::one() - total_fee;

      let fees = vec![
         (msg.fees.fee_platform.address, msg.fees.fee_platform.fee),
         (msg.fees.fee_nami.address, msg.fees.fee_nami.fee),
         (msg.fees.fee_ref.address, msg.fees.fee_ref.fee),
      ];

      Ok(Self {
         owner: msg.owner,
         ticket_denom: msg.ticket_denom,
         ticket_amount: msg.ticket_amount,
         duration_seconds: msg.duration_seconds,
         game_delay: msg.game_delay,
         contracts: msg.contracts,
         donation_addrs: msg.donation_addrs,
         winner_share,
         fees,
      })
   }

   pub fn load(storage: &dyn Storage) -> StdResult<Self> {
      CONFIG.load(storage)
   }

   pub fn validate(&self, api: &dyn Api) -> Result<(), ContractError> {
      //address validations
      api.addr_validate(self.owner.as_str())?;
      api.addr_validate(self.contracts.referral.as_str())?;
      api.addr_validate(self.contracts.swap.as_str())?;
      for addr in &self.donation_addrs {
         api.addr_validate(addr.as_str())?;
      }
      for fee in &self.fees {
         api.addr_validate(fee.0.as_str())?;
      }
      ensure!(
         self.duration_seconds.gt(&0u64),
         ContractError::Invalid("duration_seconds".to_string())
      );
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

      if let Some(contracts) = msg.contracts {
         self.contracts = contracts;
      }

      if let Some(donation_addrs) = msg.donation_addrs {
         self.donation_addrs = donation_addrs;
      }

      if let Some(game_delay) = msg.game_delay {
         self.game_delay = game_delay;
      }

      if let Some(fees) = msg.fees {
         self.fees = vec![
            (fees.fee_platform.address, fees.fee_platform.fee),
            (fees.fee_nami.address, fees.fee_nami.fee),
            (fees.fee_ref.address, fees.fee_ref.fee),
         ];
         let total_fee = fees.fee_platform.fee + fees.fee_nami.fee + fees.fee_ref.fee;
         ensure!(total_fee.lt(&Decimal::one()), ContractError::Invalid("fees_amounts".to_string()));
         self.winner_share = Decimal::one() - total_fee;
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
   pub game_delay: Option<u64>,
   pub contracts: Option<Contracts>,
   pub donation_addrs: Option<Vec<Addr>>,
   pub admins: Option<Vec<Addr>>,
   pub fees: Option<Fees>,
}

#[cfg(test)]
mod tests {
   use super::*;
   use crate::msg::{Contracts, Fee, Fees};
   use cosmwasm_std::{
      testing::{mock_dependencies, MockApi},
      Addr, Decimal, Timestamp, Uint128,
   };

   #[test]
   fn test_new_config() {
      let msg = InstantiateMsg {
         owner: Addr::unchecked("owner"),
         ticket_denom: "utoken".to_string(),
         ticket_amount: Uint128::new(100),
         duration_seconds: 3600,
         contracts: Contracts {
            referral: Addr::unchecked("referral_contract"),
            swap: Addr::unchecked("swap_contract"),
         },
         donation_addrs: vec![Addr::unchecked("donation1"), Addr::unchecked("donation2")],
         fees: Fees {
            fee_platform: Fee { address: Addr::unchecked("platform"), fee: Decimal::percent(10) },
            fee_nami: Fee { address: Addr::unchecked("nami"), fee: Decimal::percent(5) },
            fee_ref: Fee { address: Addr::unchecked("referral"), fee: Decimal::percent(3) },
         },
         starts_at: Timestamp::from_seconds(0),
         game_delay: 60,
         admins: None,
      };

      let config = Config::new(msg).unwrap();
      assert_eq!(config.owner, Addr::unchecked("owner"));
      assert_eq!(config.ticket_denom, "utoken");
      assert_eq!(config.ticket_amount, Uint128::new(100));
      assert_eq!(config.duration_seconds, 3600);
      assert_eq!(config.donation_addrs.len(), 2);
      assert_eq!(config.fees.len(), 3);
      assert_eq!(config.winner_share, Decimal::percent(82)); // 100% - 10% - 5% - 3%
   }

   #[test]
   fn test_new_config_invalid_fees() {
      let msg = InstantiateMsg {
         owner: Addr::unchecked("owner"),
         ticket_denom: "utoken".to_string(),
         ticket_amount: Uint128::new(100),
         duration_seconds: 3600,
         contracts: Contracts {
            referral: Addr::unchecked("referral_contract"),
            swap: Addr::unchecked("swap_contract"),
         },
         donation_addrs: vec![],
         fees: Fees {
            fee_platform: Fee { address: Addr::unchecked("platform"), fee: Decimal::percent(50) },
            fee_nami: Fee { address: Addr::unchecked("nami"), fee: Decimal::percent(30) },
            fee_ref: Fee { address: Addr::unchecked("referral"), fee: Decimal::percent(25) },
         },
         starts_at: Timestamp::from_seconds(0),
         game_delay: 60,
         admins: None,
      };

      let config = Config::new(msg);
      assert!(config.is_err()); // Fees add up to more than 100%, so it should fail
   }

   #[test]
   fn test_validate_config() {
      let api = MockApi::default();
      let msg = InstantiateMsg {
         owner: api.addr_make("owner"),
         ticket_denom: "utoken".to_string(),
         ticket_amount: Uint128::new(100),
         duration_seconds: 3600,
         contracts: Contracts {
            referral: api.addr_make("referral_contract"),
            swap: api.addr_make("swap_contract"),
         },
         donation_addrs: vec![api.addr_make("donation1")],
         fees: Fees {
            fee_platform: Fee { address: api.addr_make("platform"), fee: Decimal::percent(10) },
            fee_nami: Fee { address: api.addr_make("nami"), fee: Decimal::percent(5) },
            fee_ref: Fee { address: api.addr_make("referral"), fee: Decimal::percent(3) },
         },
         starts_at: Timestamp::from_seconds(0),
         game_delay: 60,
         admins: None,
      };

      let config = Config::new(msg).unwrap();
      let validation_result = config.validate(&api);
      assert!(validation_result.is_ok())
   }

   #[test]
   fn test_apply_update() {
      let msg = InstantiateMsg {
         owner: Addr::unchecked("owner"),
         ticket_denom: "utoken".to_string(),
         ticket_amount: Uint128::new(100),
         duration_seconds: 3600,
         contracts: Contracts {
            referral: Addr::unchecked("referral_contract"),
            swap: Addr::unchecked("swap_contract"),
         },
         donation_addrs: vec![Addr::unchecked("donation1"), Addr::unchecked("donation2")],
         fees: Fees {
            fee_platform: Fee { address: Addr::unchecked("platform"), fee: Decimal::percent(10) },
            fee_nami: Fee { address: Addr::unchecked("nami"), fee: Decimal::percent(5) },
            fee_ref: Fee { address: Addr::unchecked("referral"), fee: Decimal::percent(3) },
         },
         starts_at: Timestamp::from_seconds(0),
         game_delay: 60,
         admins: None,
      };

      let mut config = Config::new(msg).unwrap();

      let update = ConfigUpdate {
         owner: Some(Addr::unchecked("new_owner")),
         ticket_denom: Some("newtoken".to_string()),
         ticket_amount: Some(Uint128::new(200)),
         duration_seconds: None,
         game_delay: None,
         contracts: None,
         donation_addrs: None,
         fees: Some(Fees {
            fee_platform: Fee { address: Addr::unchecked("platform"), fee: Decimal::percent(8) },
            fee_nami: Fee { address: Addr::unchecked("nami"), fee: Decimal::percent(4) },
            fee_ref: Fee { address: Addr::unchecked("referral"), fee: Decimal::percent(2) },
         }),
         admins: None,
      };

      config.apply_update(update).unwrap();
      assert_eq!(config.owner, Addr::unchecked("new_owner"));
      assert_eq!(config.ticket_denom, "newtoken");
      assert_eq!(config.ticket_amount, Uint128::new(200));
      assert_eq!(config.duration_seconds, 3600); // Unchanged
      assert_eq!(config.fees.len(), 3);
      assert_eq!(config.winner_share, Decimal::percent(86)); // Updated to 100% - 8% - 4% - 2%
   }

   #[test]
   fn test_save_and_load_config() {
      let mut deps = mock_dependencies();
      let msg = InstantiateMsg {
         owner: Addr::unchecked("owner"),
         ticket_denom: "utoken".to_string(),
         ticket_amount: Uint128::new(100),
         duration_seconds: 3600,
         contracts: Contracts {
            referral: Addr::unchecked("referral_contract"),
            swap: Addr::unchecked("swap_contract"),
         },
         donation_addrs: vec![Addr::unchecked("donation1")],
         fees: Fees {
            fee_platform: Fee { address: Addr::unchecked("platform"), fee: Decimal::percent(10) },
            fee_nami: Fee { address: Addr::unchecked("nami"), fee: Decimal::percent(5) },
            fee_ref: Fee { address: Addr::unchecked("referral"), fee: Decimal::percent(3) },
         },
         starts_at: Timestamp::from_seconds(0),
         game_delay: 60,
         admins: None,
      };

      let config = Config::new(msg).unwrap();
      config.save(&mut deps.storage).unwrap();

      let loaded_config = Config::load(&deps.storage).unwrap();
      assert_eq!(config, loaded_config);
   }
}
