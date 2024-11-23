use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Decimal, Timestamp, Uint128};
use wenruji_rs::DecayGame;

use crate::config::{Config, ConfigUpdate};

#[cw_serde]
pub struct InstantiateMsg {
   pub owner: Addr,
   pub ticket_denom: String,
   pub ticket_amount: Uint128,
   pub starts_at: Timestamp,
   pub duration_seconds: u64,
   pub game_delay: u64,
   pub contracts: Contracts,
   pub donation_addrs: Vec<Addr>,
   pub admins: Option<Vec<Addr>>,
   pub fees: Fees,
}

#[cw_serde]
pub enum ExecuteMsg {
   Join { ref_code: Option<String> },
   Donate {},
   Exit {},
   EndGame { winner: Addr, restart: bool },
   Restart {},
   UpdateConfig { new_config: ConfigUpdate },
   Callback(kujira::CallbackMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
   #[returns(DecayGame)]
   GameStatus {},

   #[returns(bool)]
   HasJoined { player: Addr },

   #[returns(bool)]
   HasExited { player: Addr },

   #[returns(Vec<Coin>)]
   Donations {},

   #[returns(Decimal)]
   RefWeight { player: Addr },

   #[returns(Config)]
   Config {},
}

#[cw_serde]
pub enum CallbackType {
   PostSwap { winner: Addr, restart: bool },
}

#[cw_serde]
pub struct Contracts {
   pub swap: Addr,
   pub referral: Addr,
}

#[cw_serde]
pub struct Fees {
   pub fee_platform: Fee,
   pub fee_nami: Fee,
   pub fee_ref: Fee,
}

#[cw_serde]
pub struct Fee {
   pub address: Addr,
   pub fee: Decimal,
}
