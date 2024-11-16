use std::vec;

use cosmwasm_std::{
   coins, ensure, wasm_execute, Addr, Api, BankMsg, Binary, Event, QuerierWrapper, Response,
   Storage, Timestamp, WasmMsg,
};
use cw_storage_plus::{Item, Map};
use wenruji_rs::{calculate_fee_distribution, to_addr};

use crate::{config::Config, game::GameSM, ContractError};

pub const GAME_SM: GameSM = GameSM::new();
pub static IDX: Item<u64> = Item::new("game_idx");
pub const SNAPSHOT: Map<u64, Binary> = Map::new("snap");

pub fn execute_ref(
   api: &dyn Api,
   storage: &mut dyn Storage,
   querier: QuerierWrapper,
   config: &Config,
   account: &Addr,
   ref_code: Option<String>,
) -> Result<(Option<String>, Option<WasmMsg>), ContractError> {
   let ambassador: String = querier.query_wasm_smart(
      config.fees.fee_ref.address.clone(),
      &referral::QueryMsg::GetReferrer { user: account.clone() },
   )?;
   if !ambassador.is_empty() {
      let ambassador_addr = to_addr(ambassador.clone(), api)?;
      GAME_SM.increase_ref(storage, &ambassador_addr)?;
      return Ok((Some(ambassador), None));
   } else {
      if let Some(ref_code) = ref_code {
         // Query the referral contract to get the referrer address
         let ambassador: String = querier.query_wasm_smart(
            config.fees.fee_ref.address.clone(),
            &referral::QueryMsg::GetAddr { code: ref_code.clone() },
         )?;
         let ambassador_addr = to_addr(ambassador.clone(), api)?;
         GAME_SM.increase_ref(storage, &ambassador_addr)?;

         let msg = wasm_execute(
            config.fees.fee_ref.address.clone(),
            &referral::ExecuteMsg::AddReferee { referee: account.clone(), code: ref_code },
            vec![],
         )?;
         return Ok((Some(ambassador.clone()), Some(msg)));
      }
      return Ok((None, None));
   }
}

pub fn execute_restart(
   storage: &mut dyn Storage,
   now: Timestamp,
   duration: u64,
   delay: u64,
) -> Result<u64, ContractError> {
   ensure!(GAME_SM.is_completed(storage, now)?, ContractError::GameNotEnded {});
   let snap = GAME_SM.get_snap(storage)?;
   let idx = IDX.load(storage)?;
   if idx.gt(&10u64) {
      let remove_idx = idx - 10u64;
      SNAPSHOT.remove(storage, remove_idx);
      SNAPSHOT.save(storage, idx, &snap)?;
   } else {
      SNAPSHOT.save(storage, idx, &snap)?;
   }
   let start = now.plus_seconds(delay);
   GAME_SM.restart(storage, start, start.plus_seconds(duration))?;
   IDX.save(storage, &(idx + 1u64))?;
   Ok(idx + 1u64)
}

pub fn execute_endgame(
   storage: &mut dyn Storage,
   now: Timestamp,
   config: &Config,
) -> Result<Response, ContractError> {
   let mut response = Response::new();
   let (curr, amount) = GAME_SM.endgame(storage, now)?;

   let (winner, points) = curr.unwrap(); //safe unwrap endgame if one participant exist the winner exist otherwise endgame throw error norewards

   let ref_weights = GAME_SM.get_ref_weights(storage)?;

   let mut fees = vec![
      (winner.clone(), config.winner_share),
      (config.fees.fee_platform.address.clone(), config.fees.fee_platform.bp),
   ];

   if !ref_weights.is_empty() {
      fees.push((config.fees.fee_ref.address.clone(), config.fees.fee_ref.bp));
   }

   let fee_split =
      calculate_fee_distribution(coins(amount.into(), config.ticket_denom.clone()), &fees);

   response = response.add_messages(vec![
      BankMsg::Send { to_address: fee_split[0].0.to_string(), amount: fee_split[0].1.clone() },
      BankMsg::Send { to_address: fee_split[1].0.to_string(), amount: fee_split[1].1.clone() },
   ]);

   if !ref_weights.is_empty() {
      response = response.add_message(wasm_execute(
         config.fees.fee_ref.address.clone(),
         &referral::ExecuteMsg::DistributeRewards { referers: ref_weights },
         fee_split[2].1.clone(),
      )?);
   }

   Ok(response
      .add_event(Event::new("hitnrug/endgame"))
      .add_attribute("winner", winner)
      .add_attribute("points", points.to_string()))
}
