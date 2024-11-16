use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Timestamp, Uint128};

use crate::{
   config::{Config, ConfigUpdate},
   game::GameSmSnapshot,
};

#[cw_serde]
pub struct InstantiateMsg {
   pub owner: Addr,
   pub ticket_denom: String,
   pub ticket_amount: Uint128,
   pub starts_at: Timestamp,
   pub duration_seconds: u64,
   pub game_delay_sec: u64,
   pub delay_play_seconds: u64,
   pub fees: Fees,
   pub points: Points,
}

#[cw_serde]
pub enum ExecuteMsg {
   Join { ref_code: Option<String> },
   Exit {},
   Play(PlayMsg),
   EndGame {},
   Restart {},
   UpdateConfig { new_config: ConfigUpdate },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
   #[returns(GameSmSnapshot)]
   GameStatus { idx: Option<u64> },

   #[returns(Config)]
   Config {},

   #[returns(Uint128)]
   GameIndex {},
}

#[cw_serde]
pub struct Fees {
   pub fee_platform: Fee,
   pub fee_ref: Fee,
}

#[cw_serde]
pub struct Fee {
   pub address: Addr,
   pub bp: Decimal,
}

#[cw_serde]
pub enum PlayMsg {
   Keep {},
   Hit { target: Addr },
   Help { target: Addr },
}

#[cw_serde]
pub struct Points {
   pub keep: i64,
   pub hit: i64,
   pub help: Point,
}

#[cw_serde]
pub struct Point {
   pub myself: i64,
   pub other: i64,
}
