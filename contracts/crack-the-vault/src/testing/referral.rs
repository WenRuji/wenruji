use crate::ContractError;
use cosmwasm_std::{
   entry_point, to_json_binary, Addr, Api, Binary, Deps, DepsMut, Env, Event, MessageInfo,
   Response, StdError,
};
use cw2::set_contract_version;
use referral::{ExecuteMsg, InstantiateMsg, QueryMsg};
use wenruji_rs::to_addr;

// version info for migration info
const CONTRACT_NAME: &str = "referral";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
   deps: DepsMut,
   _: Env,
   _info: MessageInfo,
   _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
   set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
   Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
   _deps: DepsMut,
   _env: Env,
   info: MessageInfo,
   msg: ExecuteMsg,
) -> Result<Response, ContractError> {
   match msg {
      ExecuteMsg::GenCode { .. } => todo!(),
      ExecuteMsg::AddReferee { code: _, referee: _ } => {
         Ok(Response::default().add_event(Event::new("referral/add_referee")))
      }
      ExecuteMsg::UpdateConfig(..) => todo!(),
      ExecuteMsg::ClaimRewards {} => todo!(),
      ExecuteMsg::DistributeRewards { referers: _ } => {
         let event = Event::new("referral/distribute_rewards").add_attributes(vec![
            ("action", "distribute-rewards"),
            ("sender", info.sender.as_str()),
         ]);

         Ok(Response::new().add_event(event))
      }
   }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
   Ok(match msg {
      QueryMsg::Config {} => todo!(),
      QueryMsg::GetCode { .. } => todo!(),
      QueryMsg::GetAddr { code } => get_addr(code, deps.api),
      QueryMsg::GetReferrer { user } => get_referrer(user, deps.api),
      QueryMsg::ReferralStructure { .. } => todo!(),
      QueryMsg::PendingRewards { .. } => todo!(),
   }?)
}

fn get_addr(code: String, api: &dyn Api) -> Result<Binary, ContractError> {
   let valid_code = "VALID_CODE".to_string();
   if code == valid_code {
      return Ok(to_json_binary(&to_addr(
         "cosmwasm1se09wrdugr8m62wwd6xgrukuvqjntf9e73p9lmexwkry35sh3v5s52j5fn".to_string(), //VALID_AMBASSADOR => To Addr
         api,
      )?)?);
   } else {
      return Err(ContractError::Std(StdError::not_found("code")));
   }
}

fn get_referrer(user: Addr, api: &dyn Api) -> Result<Binary, ContractError> {
   let valid_user = to_addr(
      "cosmwasm1rw8dag7khgvtmd5srl42vthe4ez86l475f92ruzx6d0kudc0dp4srs5gfm".to_string(),
      api,
   )?; //VALID_USER => to Addr
   if user == valid_user {
      return Ok(to_json_binary(&to_addr(
         "cosmwasm1se09wrdugr8m62wwd6xgrukuvqjntf9e73p9lmexwkry35sh3v5s52j5fn".to_string(), //VALID_AMBASSADOR => To Addr
         api,
      )?)?);
   } else {
      return Ok(to_json_binary("")?);
   }
}
