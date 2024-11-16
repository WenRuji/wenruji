use cosmwasm_std::{Addr, Api, Coin, StdError};
use cw_utils::NativeBalance;

pub fn to_addr(addr: String, api: &dyn Api) -> Result<Addr, StdError> {
   let canonical_addr = api.addr_canonicalize(&addr)?;
   api.addr_humanize(&canonical_addr)
}

pub fn normalize(coins: Vec<Coin>) -> Vec<Coin> {
   let mut coins = NativeBalance(coins);
   coins.normalize();
   coins.into_vec()
}

#[cfg(test)]
mod tests {
   use cosmwasm_std::testing::mock_dependencies;

   use super::*;

   #[test]
   fn test_addr() {
      let deps = mock_dependencies();

      let addr = deps.api.addr_make("test");
      let verify = to_addr(addr.to_string(), &deps.api).unwrap();

      assert_eq!(addr, verify)
   }
}
