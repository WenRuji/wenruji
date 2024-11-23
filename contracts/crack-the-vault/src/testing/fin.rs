use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
   coins, entry_point, to_json_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Decimal256, Deps,
   DepsMut, Empty, Env, MessageInfo, Response, StdError, StdResult, Uint128, Uint256,
};
use kujira::CallbackData;
use referral::InstantiateMsg;

use crate::ContractError;

#[entry_point]
pub fn fin_instantiate(
   _deps: DepsMut,
   _env: Env,
   _info: MessageInfo,
   _msg: InstantiateMsg,
) -> StdResult<Response> {
   Ok(Response::new())
}

#[cw_serde]
pub enum FinExecuteMsg {
   Swap {
      /// Field provided for backward compatibility but ignored. Only a single
      /// asset may be provided for a swap
      offer_asset: Option<Coin>,
      belief_price: Option<Decimal256>,
      max_spread: Option<Decimal256>,
      to: Option<Addr>,

      /// An optional callback that FIN will execute with the funds from the swap.
      /// The callback is executed on the sender's address.
      /// NB: This is currently pre-release, and not yet available on production contracts
      #[serde(skip_serializing_if = "Option::is_none")]
      callback: Option<CallbackData>,
   },
}

const USDC: &str = "kujira15drytn4ntvg7f292ncul6wcxle4xe404q280hcw9878zjqa2h9nqulj2e9";
const USK: &str = "kujira1v4h0t7dfguwg927y6zz496wxc88lc96ekl6fnc3r5yqty9snlrnsm0ee6t";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn fin_execute(
   mut _deps: DepsMut,
   _env: Env,
   info: MessageInfo,
   msg: FinExecuteMsg,
) -> Result<Response, ContractError> {
   match msg {
      FinExecuteMsg::Swap { offer_asset: _, belief_price, max_spread: _, to: _, callback } => {
         let coin = info.funds[0].clone();
         let amount: Uint256 = coin.amount.into();
         let price = belief_price.unwrap_or_else(|| Decimal256::from_ratio(200u128, 100u128));
         let (price, return_denom) = match coin.denom.as_str() {
            USDC => (price, USK),
            USK => (price, USDC),
            _ => return Err(ContractError::Std(StdError::generic_err("Invalid Denom"))),
         };

         let u256_amount = amount.mul_floor(price);
         let return_amount = Uint128::try_from(u256_amount)?;

         let message = match callback {
            Some(cb) => {
               cb.to_message(&info.sender, &Empty {}, coins(return_amount.u128(), return_denom))?
            }
            None => CosmosMsg::Bank(BankMsg::Send {
               to_address: info.sender.to_string(),
               amount: coins(return_amount.u128(), return_denom),
            }),
         };
         Ok(Response::default().add_message(message).add_attribute("action", "fin-swap"))
      }
   }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn fin_query(_deps: Deps, _env: Env, _msg: kujira::fin::QueryMsg) -> StdResult<Binary> {
   Ok(to_json_binary("")?)
}
