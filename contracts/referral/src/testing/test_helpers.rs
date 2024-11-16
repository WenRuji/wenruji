use crate::msg::*;
use cosmwasm_std::{testing::MockApi, Addr, Coin, Decimal};
use cw_multi_test::{App, AppResponse, BasicAppBuilder, ContractWrapper, Executor};
use cw_utils::NativeBalance;

pub struct TestEnv {
   pub app: App,
   pub owner: Addr,
   pub referral_addr: Addr,
   pub referral_code_id: u64,
}

pub fn setup_test_env(
   initial_balance: Vec<(Addr, Vec<Coin>)>,
   instantiate_msg: InstantiateMsg,
) -> TestEnv {
   let mut app = BasicAppBuilder::new().build(|router, _, storage| {
      for (addr, coins) in initial_balance {
         router.bank.init_balance(storage, &addr, coins).unwrap();
      }
   });
   let owner = app.api().addr_make("owner");
   let referral_code_id = app.store_code(Box::new(ContractWrapper::new(
      crate::contract::execute,
      crate::contract::instantiate,
      crate::contract::query,
   )));

   // Instantiate contract
   let referral_addr = app
      .instantiate_contract(referral_code_id, owner.clone(), &instantiate_msg, &[], "rewards", None)
      .unwrap();

   TestEnv { app, owner, referral_addr, referral_code_id }
}

pub fn create_config(
   owner: &str,
   whitelisted_contracts: Whitelist,
   whitelisted_denoms: Whitelist,
) -> InstantiateMsg {
   InstantiateMsg {
      owner: MockApi::default().addr_make(owner),
      whitelisted_denoms,
      whitelisted_contracts,
   }
}

impl TestEnv {
   pub fn addr(&self, account: &str) -> Addr {
      self.app.api().addr_make(account)
   }

   pub fn gen_code(&mut self, account: &str, code: String) -> anyhow::Result<AppResponse> {
      self.app.execute_contract(
         self.addr(account),
         self.referral_addr.clone(),
         &ExecuteMsg::GenCode { code },
         &[],
      )
   }

   pub fn assert_code(&mut self, account: &str, code: String) {
      let res: String = self
         .app
         .wrap()
         .query_wasm_smart(&self.referral_addr, &QueryMsg::GetCode { user: self.addr(account) })
         .unwrap();

      assert_eq!(res, code);
   }

   pub fn add_referee(
      &mut self,
      account: &str,
      referee: &str,
      code: String,
   ) -> anyhow::Result<AppResponse> {
      self.app.execute_contract(
         self.addr(account),
         self.referral_addr.clone(),
         &ExecuteMsg::AddReferee { referee: self.addr(referee), code },
         &[],
      )
   }

   pub fn claim_rewards(&mut self, account: &str) -> anyhow::Result<AppResponse> {
      self.app.execute_contract(
         self.addr(account),
         self.referral_addr.clone(),
         &ExecuteMsg::ClaimRewards {},
         &[],
      )
   }

   pub fn distribute_rewards(
      &mut self,
      account: &str,
      amounts: Vec<Coin>,
      referers: Vec<(Addr, Decimal)>,
   ) -> anyhow::Result<AppResponse> {
      self.app.execute_contract(
         self.addr(account),
         self.referral_addr.clone(),
         &ExecuteMsg::DistributeRewards { referers },
         &amounts,
      )
   }

   pub fn assert_balance(&self, account: &str, expected: Coin) {
      let balance = self.app.wrap().query_balance(self.addr(account), &expected.denom).unwrap();
      assert_eq!(balance, expected, "Balance mismatch for {account}: {balance} != {expected}");
   }

   pub fn assert_pending_rewards(&self, account: &str, expected: Vec<Coin>) {
      let pending_rewards: PendingRewardsResponse = self
         .app
         .wrap()
         .query_wasm_smart(
            self.referral_addr.clone(),
            &QueryMsg::PendingRewards { user: self.addr(account) },
         )
         .unwrap();
      let mut normalized = NativeBalance(expected);
      normalized.normalize();
      assert_eq!(
         pending_rewards.rewards,
         normalized.into_vec(),
         "Pending rewards mismatch for {}",
         account
      );
   }

   pub fn update_config(
      &mut self,
      account: &str,
      update: ConfigUpdate,
   ) -> anyhow::Result<AppResponse> {
      self.app.execute_contract(
         self.addr(account),
         self.referral_addr.clone(),
         &ExecuteMsg::UpdateConfig(update),
         &[],
      )
   }
}
