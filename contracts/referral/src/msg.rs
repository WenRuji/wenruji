use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Decimal};

use crate::state::config::Config;

#[cw_serde]
pub struct InstantiateMsg {
   pub owner: Addr,
   pub whitelisted_denoms: Whitelist,
   pub whitelisted_contracts: Whitelist,
}

#[cw_serde]
pub enum ExecuteMsg {
   UpdateConfig(ConfigUpdate),

   GenCode { code: String },

   AddReferee { referee: Addr, code: String },

   ClaimRewards {},

   DistributeRewards { referers: Vec<(Addr, Decimal)> },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
   #[returns(Config)]
   Config {},

   #[returns(String)]
   GetCode { user: Addr },

   #[returns(String)]
   GetAddr { code: String },

   #[returns(String)]
   GetReferrer { user: Addr },

   #[returns(Vec<String>)]
   ReferralStructure { user: Addr },

   #[returns(PendingRewardsResponse)]
   PendingRewards { user: Addr },
}

#[cw_serde]
pub struct PendingRewardsResponse {
   pub rewards: Vec<Coin>,
}

#[cw_serde]
pub enum Whitelist {
   All,
   Some(Vec<String>),
}

#[cw_serde]
pub struct ConfigUpdate {
   pub owner: Option<Addr>,
   pub whitelisted_denoms: Option<Whitelist>,
   pub whitelisted_contracts: Option<Whitelist>,
}
