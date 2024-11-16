#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
   ensure, ensure_eq, to_json_binary, to_json_string, BankMsg, Binary, Coin, Deps, DepsMut, Env,
   Event, MessageInfo, Response,
};
use cw2::set_contract_version;
use cw_utils::{must_pay, nonpayable};

use crate::config::Config;
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{execute_endgame, execute_ref, execute_restart, GAME_SM, IDX, SNAPSHOT};

// version info for migration info
const CONTRACT_NAME: &str = "hitnrug";
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
   GAME_SM.initialize(
      deps.storage,
      msg.starts_at,
      msg.starts_at.plus_seconds(msg.duration_seconds),
   )?;
   IDX.save(deps.storage, &1u64)?;
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
            !GAME_SM.is_started(deps.storage, time)?,
            ContractError::Invalid("game not started".to_string())
         );

         ensure!(
            !GAME_SM.has_joined(deps.storage, &info.sender)?,
            ContractError::Invalid("already_joined".to_string())
         );
         let mut response = Response::new();

         let (ambassador, ref_msg) =
            execute_ref(deps.api, deps.storage, deps.querier, &config, &info.sender, ref_code)?;
         if let Some(msg) = ref_msg {
            response = response.add_message(msg);
         }

         GAME_SM.join(deps.storage, time, &info.sender, amount)?;

         Ok(response.add_event(
            Event::new("hitnrug/join")
               .add_attribute("account", info.sender)
               .add_attribute("ambassador", ambassador.unwrap_or_default()),
         ))
      }
      ExecuteMsg::Exit {} => {
         nonpayable(&info)?;
         ensure!(
            GAME_SM.is_started(deps.storage, time)?,
            ContractError::Invalid("game already started".to_string())
         );

         let (amount, decay_snap) = GAME_SM.exit(deps.storage, time, &info.sender)?;

         let msg = BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![Coin::new(amount, config.ticket_denom.clone())],
         };

         Ok(Response::new().add_message(msg).add_event(
            Event::new("hitnrug/exit")
               .add_attribute("account", info.sender)
               .add_attribute("decay_snap", decay_snap.to_string()),
         ))
      }
      ExecuteMsg::Play(play_msg) => {
         nonpayable(&info)?;
         ensure!(
            GAME_SM.is_started(deps.storage, time)?,
            ContractError::Invalid("game not started".to_string())
         );
         ensure!(
            !GAME_SM.is_ended(deps.storage, time)?,
            ContractError::Invalid("game_ended".to_string())
         );
         ensure!(
            GAME_SM.has_joined(deps.storage, &info.sender)?,
            ContractError::Invalid("not_joined".to_string())
         );
         ensure!(
            !GAME_SM.has_exited(deps.storage, &info.sender)?,
            ContractError::Invalid("exited_cannot_play".to_string())
         );
         GAME_SM.play(deps.storage, play_msg.clone(), &info.sender, &config, time)?;
         let action = to_json_string(&play_msg)?;
         Ok(Response::new().add_event(Event::new("hitnrug/play").add_attribute("action", action)))
      }
      ExecuteMsg::EndGame {} => {
         nonpayable(&info)?;
         execute_endgame(deps.storage, time, &config)
      }
      ExecuteMsg::Restart {} => {
         let idx =
            execute_restart(deps.storage, time, config.duration_seconds, config.game_delay_sec)?;

         Ok(Response::new().add_event(
            Event::new("hitnrug/restart")
               .add_attribute("game_idx", idx.to_string())
               .add_attribute("game_starts_at", time.to_string())
               .add_attribute(
                  "game_ends_at",
                  (time.plus_seconds(config.duration_seconds)).to_string(),
               ),
         ))
      }
      ExecuteMsg::UpdateConfig { new_config } => {
         ensure!(info.sender == config.owner, ContractError::Unauthorized {});
         ensure!(
            GAME_SM.is_completed(deps.storage, time)?,
            ContractError::Invalid("Game Not Completed".to_string())
         );
         config.apply_update(new_config)?;
         config.validate(deps.api)?;
         config.save(deps.storage)?;
         Ok(Response::new().add_event(Event::new("hitnrug/update_config")))
      }
   }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
   match msg {
      QueryMsg::GameStatus { idx } => match idx {
         Some(idx) => Ok(SNAPSHOT.load(deps.storage, idx)?),
         None => GAME_SM.get_snap(deps.storage),
      },
      QueryMsg::Config {} => Ok(to_json_binary(&Config::load(deps.storage)?)?),
      QueryMsg::GameIndex {} => Ok(to_json_binary(&IDX.load(deps.storage)?)?),
   }
}
