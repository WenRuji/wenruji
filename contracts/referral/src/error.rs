use cosmwasm_std::StdError;
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
   #[error("{0}")]
   Std(#[from] StdError),

   #[error("{0}")]
   Payment(#[from] PaymentError),

   #[error("unauthorized")]
   Unauthorized {},

   #[error("No rewards to claim")]
   NoRewardsToClaim {},

   #[error("Reward denom not on whitelist")]
   RewardNotWhitelisted {},
}
