use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, StdResult, Storage};
use cw_storage_plus::Item;

use crate::msg::{ConfigUpdate, InstantiateMsg, Whitelist};

use crate::ContractError;

#[cw_serde]
pub struct Config {
   pub owner: Addr,
   pub whitelisted_denoms: Whitelist,
   pub whitelisted_contracts: Whitelist,
}

impl Config {
   pub fn load(storage: &dyn Storage) -> StdResult<Self> {
      Item::new("config").load(storage)
   }

   pub fn save(&self, storage: &mut dyn Storage, api: &dyn Api) -> Result<(), ContractError> {
      self.validate(api)?;
      Ok(Item::new("config").save(storage, self)?)
   }

   pub fn validate(&self, api: &dyn Api) -> Result<(), ContractError> {
      api.addr_validate(self.owner.as_str())?;

      Ok(())
   }

   pub fn apply_update(&mut self, msg: ConfigUpdate) -> Result<(), ContractError> {
      if let Some(owner) = msg.owner {
         self.owner = owner;
      }

      if let Some(whitelisted_denoms) = msg.whitelisted_denoms {
         self.whitelisted_denoms = whitelisted_denoms;
      }

      if let Some(whitelisted_contracts) = msg.whitelisted_contracts {
         self.whitelisted_contracts = whitelisted_contracts;
      }

      Ok(())
   }
}

impl From<InstantiateMsg> for Config {
   fn from(msg: InstantiateMsg) -> Self {
      Self {
         owner: msg.owner,
         whitelisted_denoms: msg.whitelisted_denoms,
         whitelisted_contracts: msg.whitelisted_contracts,
      }
   }
}
