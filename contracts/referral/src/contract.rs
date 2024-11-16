use cosmwasm_std::{
   ensure, entry_point, to_json_binary, BankMsg, Binary, Coin, Deps, DepsMut, Env, Event,
   MessageInfo, Response,
};
use cw2::set_contract_version;
use cw_utils::PaymentError;
use wenruji_rs::{calculate_fee_distribution, RewardsSM};

use crate::{
   msg::{PendingRewardsResponse, Whitelist},
   state::{config::Config, referral::ReferralSM},
   ContractError, ExecuteMsg, InstantiateMsg, QueryMsg,
};

// version info for migration info
const CONTRACT_NAME: &str = "referral";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const REWARDS: RewardsSM = RewardsSM::new();
pub const REFERRAL: ReferralSM = ReferralSM::new();

#[entry_point]
pub fn instantiate(
   deps: DepsMut,
   _: Env,
   _info: MessageInfo,
   msg: InstantiateMsg,
) -> Result<Response, ContractError> {
   set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

   let config = Config::from(msg);
   config.save(deps.storage, deps.api)?;

   Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
   deps: DepsMut,
   _env: Env,
   info: MessageInfo,
   msg: ExecuteMsg,
) -> Result<Response, ContractError> {
   let mut config: Config = Config::load(deps.storage)?;
   match msg {
      ExecuteMsg::GenCode { code } => {
         REFERRAL.gen_code(deps.storage, &info.sender.to_string(), &code)?;
         Ok(Response::default().add_event(Event::new("referral/gen_code")))
      }
      ExecuteMsg::AddReferee { code, referee } => {
         if let Whitelist::Some(whitelist) = &config.whitelisted_contracts {
            ensure!(whitelist.contains(&info.sender.to_string()), ContractError::Unauthorized {});
         }
         REFERRAL.add_referee(deps.storage, &referee.to_string(), &code)?;
         Ok(Response::default().add_event(Event::new("referral/add_referee")))
      }
      ExecuteMsg::UpdateConfig(msg) => {
         ensure!(info.sender == config.owner, ContractError::Unauthorized {});
         config.apply_update(msg)?;
         config.save(deps.storage, deps.api)?;
         Ok(Response::default())
      }
      ExecuteMsg::ClaimRewards {} => {
         let coins: Vec<Coin> = REWARDS.claim_accrued(deps.storage, &info.sender)?;
         ensure!(!coins.is_empty(), ContractError::NoRewardsToClaim {});

         let return_msg: BankMsg =
            BankMsg::Send { to_address: info.sender.to_string(), amount: coins.clone() };

         let event = Event::new(format!("referral/claim"))
            .add_attributes(vec![("action", "claim"), ("staker", info.sender.as_str())]);

         Ok(Response::new().add_message(return_msg).add_event(event))
      }
      ExecuteMsg::DistributeRewards { referers } => {
         if let Whitelist::Some(whitelist) = &config.whitelisted_contracts {
            ensure!(whitelist.contains(&info.sender.to_string()), ContractError::Unauthorized {});
         }

         ensure!(!info.funds.is_empty(), PaymentError::NoFunds {});

         if let Whitelist::Some(whitelist) = &config.whitelisted_denoms {
            for Coin { denom, .. } in info.funds.iter() {
               ensure!(whitelist.contains(denom), ContractError::RewardNotWhitelisted {});
            }
         }

         // Distribution split
         let distribution = calculate_fee_distribution(info.funds, &referers);

         for (user, rewards) in distribution.iter() {
            REWARDS.add_accrued_rewards(deps.storage, &user, rewards)?;
         }

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
   let config = Config::load(deps.storage)?;
   Ok(match msg {
      QueryMsg::Config {} => to_json_binary(&config),
      QueryMsg::GetCode { user } => {
         to_json_binary(&REFERRAL.get_code(deps.storage, &user.to_string())?)
      }
      QueryMsg::GetAddr { code } => to_json_binary(&REFERRAL.get_addr(deps.storage, &code)?),
      QueryMsg::GetReferrer { user } => {
         to_json_binary(&REFERRAL.get_referrer(deps.storage, &user.to_string())?)
      }
      QueryMsg::ReferralStructure { user } => {
         to_json_binary(&REFERRAL.get_referral_struct(deps.storage, &user.to_string())?)
      }
      QueryMsg::PendingRewards { user } => {
         let accrued = REWARDS.get_accrued(deps.storage, &user)?;
         to_json_binary(&PendingRewardsResponse { rewards: accrued })
      }
   }?)
}
