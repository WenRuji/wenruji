#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
   ensure, ensure_eq, to_json_binary, wasm_execute, BankMsg, Binary, Coin, Decimal, Deps, DepsMut,
   Empty, Env, Event, MessageInfo, Response,
};
use cw2::set_contract_version;
use cw_utils::{must_pay, nonpayable, one_coin, PaymentError};
use kujira::CallbackData;
use wenruji_rs::{DecayGame, DecayGameError};

use crate::config::Config;
use crate::error::ContractError;
use crate::msg::{CallbackType, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{
   execute_donate, execute_endgame, execute_exit, execute_join, execute_post_swap, execute_ref,
   execute_restart, ACCOUNTS, ADMIN, DECAY_GAME, REF_WEIGHTS, REWARDS,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:crack-the-vault";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
   deps: DepsMut,
   _env: Env,
   _info: MessageInfo,
   msg: InstantiateMsg,
) -> Result<Response, ContractError> {
   set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

   let config = Config::new(msg.clone())?;
   config.validate(deps.api)?;
   config.save(deps.storage)?;

   let decay_game: DecayGame =
      DecayGame::new(msg.starts_at, msg.starts_at.plus_seconds(msg.duration_seconds));
   DECAY_GAME.save(deps.storage, &decay_game)?;

   ADMIN.save(deps.storage, &msg.owner, &Empty {})?;
   if let Some(admins) = msg.admins {
      for admin in admins {
         deps.api.addr_validate(admin.as_str())?;
         ADMIN.save(deps.storage, &admin, &Empty {})?;
      }
   }

   Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
   deps: DepsMut,
   env: Env,
   info: MessageInfo,
   msg: ExecuteMsg,
) -> Result<Response, ContractError> {
   let mut config = Config::load(deps.storage)?;
   let time = env.block.time;
   match msg {
      ExecuteMsg::Join { ref_code } => {
         let amount = must_pay(&info, &config.ticket_denom)?;
         ensure_eq!(amount, config.ticket_amount, ContractError::InsufficientFunds {});

         ensure!(
            !ACCOUNTS.has(deps.storage, info.sender.clone()),
            ContractError::Invalid("already_joined".to_string())
         );
         let mut response = Response::new();

         let (ambassador, ref_msg) =
            execute_ref(deps.api, deps.storage, deps.querier, &config, &info.sender, ref_code)?;
         if let Some(msg) = ref_msg {
            response = response.add_message(msg);
         }

         execute_join(deps.storage, time, &info.sender, amount)?;

         Ok(response.add_event(
            Event::new("crack-the-valut/join")
               .add_attribute("account", info.sender)
               .add_attribute("ambassador", ambassador.unwrap_or_default()),
         ))
      }
      ExecuteMsg::Exit {} => {
         nonpayable(&info)?;
         //amount cannot be zero
         let (amount, decay_snap) = execute_exit(deps.storage, time, &info.sender)?;

         let msg = BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![Coin::new(amount, config.ticket_denom.clone())],
         };

         Ok(Response::new().add_message(msg).add_event(
            Event::new("crack-the-valut/exit")
               .add_attribute("account", info.sender)
               .add_attribute("decay_snap", decay_snap.to_string()),
         ))
      }
      ExecuteMsg::EndGame { winner, restart } => {
         nonpayable(&info)?;
         ensure!(ADMIN.has(deps.storage, &info.sender), ContractError::Unauthorized {});

         let end_game = execute_endgame(deps.storage, time);
         match end_game {
            Ok(amount) => {
               let cb = CallbackType::PostSwap { winner: winner.clone(), restart };
               let cb_data = to_json_binary(&cb)?;

               let swap_msg = kujira::fin::ExecuteMsg::Swap {
                  offer_asset: Some(Coin { denom: config.ticket_denom.clone(), amount }),
                  belief_price: None,
                  max_spread: None,
                  to: None,
                  callback: Some(CallbackData(cb_data)),
               };

               let wasm_msg = wasm_execute(
                  config.contracts.swap,
                  &swap_msg,
                  vec![Coin::new(amount, config.ticket_denom.clone())],
               )?;
               return Ok(Response::new().add_message(wasm_msg).add_event(
                  Event::new("crack-the-valut/end_game")
                     .add_attribute("winner", winner)
                     .add_attribute("prize_amount_before", amount)
                     .add_attribute("prize_denom_before", config.ticket_denom.clone()),
               ));
            }
            Err(ContractError::DecayGameError(DecayGameError::NoRewards {})) => {
               let restart_msg =
                  wasm_execute(env.contract.address, &ExecuteMsg::Restart {}, vec![])?;
               return Ok(Response::new().add_message(restart_msg).add_event(
                  Event::new("crack-the-valut/end_game")
                     .add_attribute("winner", "")
                     .add_attribute("prize_amount_before", "")
                     .add_attribute("prize_denom_before", ""),
               ));
            }

            Err(err) => return Err(err),
         }
      }
      ExecuteMsg::Donate {} => {
         let decay_game = DECAY_GAME.load(deps.storage)?;

         ensure!(
            time.lt(&decay_game.decay_ends_at),
            ContractError::Invalid("Game Ended".to_string())
         );

         ensure!(config.donation_addrs.contains(&info.sender), ContractError::Unauthorized {});

         ensure!(!info.funds.is_empty(), ContractError::Payment(PaymentError::NoFunds {}));

         for coin in info.funds {
            execute_donate(deps.storage, coin)?;
         }

         Ok(Response::new()
            .add_event(Event::new("crack-the-valut/donate").add_attribute("sender", info.sender)))
      }
      ExecuteMsg::Restart {} => {
         execute_restart(deps.storage, time, &config)?;

         Ok(Response::new().add_event(
            Event::new("crack-the-valut/restart")
               .add_attribute("game_starts_at", time.to_string())
               .add_attribute(
                  "game_ends_at",
                  (time.plus_seconds(config.duration_seconds)).to_string(),
               ),
         ))
      }
      ExecuteMsg::UpdateConfig { new_config } => {
         ensure_eq!(info.sender, config.owner, ContractError::Unauthorized {});

         let decay_game: DecayGame = DECAY_GAME.load(deps.storage)?;

         if !(decay_game.rewards == decay_game.total - decay_game.exited) {
            return Err(ContractError::GameNotEnded {});
         }

         if let Some(admins) = new_config.admins.clone() {
            ADMIN.clear(deps.storage);
            for admin in admins {
               deps.api.addr_validate(admin.as_str())?;
               ADMIN.save(deps.storage, &admin, &Empty {})?;
            }
         }

         config.apply_update(new_config)?;
         config.validate(deps.api)?;
         config.save(deps.storage)?;
         Ok(Response::new().add_event(Event::new("crack-the-valut/update_config")))
      }
      ExecuteMsg::Callback(cb) => {
         let msg: CallbackType = cb.deserialize_callback()?;
         match msg {
            CallbackType::PostSwap { winner, restart } => {
               ensure!(
                  info.sender == config.contracts.swap,
                  ContractError::Invalid("sender".to_string())
               );
               let coin = one_coin(&info)?;
               let mut response = execute_post_swap(deps.storage, &config, winner, coin.clone())?;

               if restart {
                  response = response.add_message(wasm_execute(
                     env.contract.address,
                     &ExecuteMsg::Restart {},
                     vec![],
                  )?)
               }

               Ok(response.add_event(
                  Event::new("crack-the-valut/post_swap")
                     .add_attribute("prize_amount_after", coin.amount)
                     .add_attribute("prize_denom_after", coin.denom.clone()),
               ))
            }
         }
      }
   }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
   match msg {
      QueryMsg::GameStatus {} => Ok(to_json_binary(&DECAY_GAME.may_load(deps.storage)?)?),
      QueryMsg::HasJoined { player } => Ok(to_json_binary(&ACCOUNTS.has(deps.storage, player))?),
      QueryMsg::HasExited { player } => {
         let account = ACCOUNTS.load(deps.storage, player)?;
         Ok(to_json_binary(&(account.decay_snapshot != Decimal::zero()))?)
      }
      QueryMsg::Donations {} => {
         let rewards: Vec<Coin> = REWARDS
            .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
            .filter_map(|item| item.ok().map(|(_, coin)| coin))
            .collect();
         Ok(to_json_binary(&rewards)?)
      }
      QueryMsg::RefWeight { player } => {
         Ok(to_json_binary(&REF_WEIGHTS.may_load(deps.storage, player)?)?)
      }
      QueryMsg::Config {} => Ok(to_json_binary(&Config::load(deps.storage)?)?),
   }
}

#[cfg(test)]
mod tests {}
