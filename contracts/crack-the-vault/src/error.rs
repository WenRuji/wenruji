use cosmwasm_std::{ConversionOverflowError, OverflowError, StdError};
use cw_utils::PaymentError;
use thiserror::Error;
use wenruji_rs::DecayGameError;

#[derive(Error, Debug)]
pub enum ContractError {
   #[error("{0}")]
   Std(#[from] StdError),

   #[error("{0}")]
   Payment(#[from] PaymentError),

   #[error("{0}")]
   Overflow(#[from] OverflowError),

   #[error("{0}")]
   ConversionOverflowError(#[from] ConversionOverflowError),

   #[error("Unauthorized")]
   Unauthorized {},

   #[error("InsufficientFunds")]
   InsufficientFunds {},

   #[error("GameNotEnded")]
   GameNotEnded {},

   #[error("Invalid: {0}")]
   Invalid(String),

   #[error("{0}")]
   DecayGameError(#[from] DecayGameError),
   // Add any other custom errors you like here.
   // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}
