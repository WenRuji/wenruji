use cosmwasm_std::{
   coin, testing::MockApi, to_json_string, Addr, Coin, Decimal, Timestamp, Uint128,
};
use cw_multi_test::{App, AppResponse, BasicAppBuilder, ContractWrapper, Executor};
use referral::{self};

use crate::{
   //config::ConfigUpdate,
   contract::{execute, instantiate, query},
   msg::{Contracts, ExecuteMsg, Fee, Fees, InstantiateMsg},
};

pub struct TestEnv {
   pub app: App,
   pub owner: Addr,
   pub contracts: MockContracts,
}

pub struct MockContracts {
   pub game: Addr,
   pub fin: Addr,
   pub referral: Addr,
}

pub struct PartialInstantiate {
   pub owner: Addr,
   pub ticket_denom: String,
   pub ticket_amount: Uint128,
   pub starts_at: Timestamp,
   pub duration_seconds: u64,
   pub game_delay: u64,
   pub donation_addrs: Vec<Addr>,
   pub fees: Vec<Decimal>,
}

pub fn setup_test_env(balances: Vec<(Addr, Vec<Coin>)>, config: PartialInstantiate) -> TestEnv {
   let mut app = BasicAppBuilder::new().build(|_router, _, _storage| {});

   let fin_code = app.store_code(Box::new(ContractWrapper::new(
      crate::testing::fin::fin_execute,
      crate::testing::fin::fin_instantiate,
      crate::testing::fin::fin_query,
   )));

   //Instantiate REFERRAL CONTRACT
   let fin_addr = app
      .instantiate_contract(
         fin_code,
         app.api().addr_make("owner"),
         &referral::InstantiateMsg {
            owner: app.api().addr_make("owner"),
            whitelisted_denoms: referral::msg::Whitelist::All,
            whitelisted_contracts: referral::msg::Whitelist::All,
         },
         &[],
         "fin",
         None,
      )
      .unwrap();

   let ref_code_id = app.store_code(Box::new(ContractWrapper::new(
      crate::testing::referral::execute,
      crate::testing::referral::instantiate,
      crate::testing::referral::query,
   )));

   //Instantiate REFERRAL CONTRACT
   let ref_addr = app
      .instantiate_contract(
         ref_code_id,
         app.api().addr_make("owner"),
         &referral::InstantiateMsg {
            owner: app.api().addr_make("owner"),
            whitelisted_denoms: referral::msg::Whitelist::All,
            whitelisted_contracts: referral::msg::Whitelist::All,
         },
         &[],
         "referral",
         None,
      )
      .unwrap();

   let game_code_id = app.store_code(Box::new(ContractWrapper::new(execute, instantiate, query)));

   let game_addr = app
      .instantiate_contract(
         game_code_id,
         app.api().addr_make("owner"),
         &InstantiateMsg {
            owner: config.owner,
            ticket_denom: config.ticket_denom,
            ticket_amount: config.ticket_amount,
            starts_at: config.starts_at,
            duration_seconds: config.duration_seconds,
            contracts: Contracts { swap: fin_addr.clone(), referral: ref_addr.clone() },
            donation_addrs: config.donation_addrs,
            game_delay: config.game_delay,
            fees: Fees {
               fee_platform: Fee { address: app.api().addr_make("swap"), fee: config.fees[0] },
               fee_nami: Fee { address: app.api().addr_make("nami"), fee: config.fees[1] },
               fee_ref: Fee { address: ref_addr.clone(), fee: config.fees[2] },
            },
            admins: None,
         },
         &[],
         "game",
         None,
      )
      .unwrap();

   app.init_modules(|router, _, storage| {
      for (addr, coins) in balances {
         router.bank.init_balance(storage, &addr, coins).unwrap();
      }
      router
         .bank
         .init_balance(
            storage,
            &fin_addr.clone(),
            vec![
               coin(
                  1_000_000_000_00u128,
                  "kujira1v4h0t7dfguwg927y6zz496wxc88lc96ekl6fnc3r5yqty9snlrnsm0ee6t",
               ),
               coin(
                  1_000_000_000_00u128,
                  "kujira15drytn4ntvg7f292ncul6wcxle4xe404q280hcw9878zjqa2h9nqulj2e9",
               ),
            ],
         )
         .unwrap();
   });

   TestEnv {
      app,
      owner: MockApi::default().addr_make("owner"),
      contracts: MockContracts { game: game_addr, referral: ref_addr, fin: fin_addr },
   }
}

pub fn create_partial_instantiate(
   owner: &str,
   ticket_denom: &str,
   ticket_amount: Uint128,
   starts_at: Timestamp,
   duration_seconds: u64,
   donation_addrs: Vec<Addr>,
   fees: Vec<Decimal>,
) -> PartialInstantiate {
   PartialInstantiate {
      owner: MockApi::default().addr_make(&owner),
      ticket_denom: ticket_denom.to_string(),
      ticket_amount,
      starts_at,
      duration_seconds,
      donation_addrs,
      fees,
      game_delay: 0u64,
   }
}

impl TestEnv {
   pub fn addr(&self, account: &str) -> Addr {
      self.app.api().addr_make(account)
   }

   pub fn donate(&mut self, account: &str, funds: Vec<Coin>) -> anyhow::Result<AppResponse> {
      self.app.execute_contract(
         self.addr(account),
         self.contracts.game.clone(),
         &ExecuteMsg::Donate {},
         &funds,
      )
   }

   pub fn join(
      &mut self,
      account: &str,
      ref_code: Option<String>,
      funds: Vec<Coin>,
   ) -> anyhow::Result<AppResponse> {
      self.app.execute_contract(
         self.addr(account),
         self.contracts.game.clone(),
         &ExecuteMsg::Join { ref_code },
         &funds,
      )
   }

   pub fn exit(&mut self, account: &str) -> anyhow::Result<AppResponse> {
      self.app.execute_contract(
         self.addr(account),
         self.contracts.game.clone(),
         &ExecuteMsg::Exit {},
         &[],
      )
   }

   pub fn restart(&mut self, account: &str) -> anyhow::Result<AppResponse> {
      self.app.execute_contract(
         self.addr(account),
         self.contracts.game.clone(),
         &ExecuteMsg::Restart {},
         &[],
      )
   }

   pub fn endgame(
      &mut self,
      account: &str,
      winner: &str,
      restart: bool,
   ) -> anyhow::Result<AppResponse> {
      self.app.execute_contract(
         self.addr(account),
         self.contracts.game.clone(),
         &ExecuteMsg::EndGame { winner: self.addr(winner), restart },
         &[],
      )
   }

   // pub fn update_config(
   //    &mut self,
   //    account: &str,
   //    new_config: ConfigUpdate,
   // ) -> anyhow::Result<AppResponse> {
   //    self.app.execute_contract(
   //       self.addr(account),
   //       self.contracts.game.clone(),
   //       &ExecuteMsg::UpdateConfig { new_config },
   //       &[],
   //    )
   // }

   pub fn move_block(&mut self, seconds: u64) {
      let mut new_block = self.app.block_info();
      new_block.time = Timestamp::from_seconds(new_block.time.seconds() + seconds);
      self.app.set_block(new_block);
   }

   pub fn set_block(&mut self, time: Timestamp) {
      let mut new_block = self.app.block_info();
      new_block.time = time;
      self.app.set_block(new_block);
   }

   pub fn assert_balance(&self, account: &str, expected: Coin) {
      let balance = self.app.wrap().query_balance(self.addr(account), &expected.denom).unwrap();
      assert_eq!(balance, expected, "Balance mismatch for {account}: {balance} != {expected}");
   }
}

#[test]
fn test_serialize_instantiate_msg() {
   // Create a sample instance of InstantiateMsg with mock data
   let msg = InstantiateMsg {
      owner: Addr::unchecked("kujira15m5jv9ttlkvchkaca72wse7v8zx7hll4x6u0cf"),
      ticket_denom:
         "factory/kujira1r85reqy6h0lu02vyz0hnzhv5whsns55gdt4w0d7ft87utzk7u0wqr4ssll/uusk"
            .to_string(),
      ticket_amount: Uint128::new(100000),
      starts_at: Timestamp::from_seconds(1730856600),
      duration_seconds: 900,
      contracts: Contracts {
         swap: Addr::unchecked("kujira1wl003xxwqltxpg5pkre0rl605e406ktmq5gnv0ngyjamq69mc2kqm06ey6"),
         referral: Addr::unchecked(
            "kujira1rxud2nlh2cayaaewj0fvuaz39mcj7xf9g3wv33gyhv428kujckuqndct66",
         ),
      },
      donation_addrs: vec![Addr::unchecked("kujira1y3ztnmghrmsa8d8h5ny7h2lvq4w3lre9hvwhcw")],
      fees: Fees {
         fee_platform: Fee {
            address: Addr::unchecked("kujira15m5jv9ttlkvchkaca72wse7v8zx7hll4x6u0cf"),
            fee: Decimal::percent(10),
         },
         fee_nami: Fee {
            address: Addr::unchecked("kujira1y3ztnmghrmsa8d8h5ny7h2lvq4w3lre9hvwhcw"),
            fee: Decimal::percent(10),
         },
         fee_ref: Fee {
            address: Addr::unchecked(
               "kujira1rxud2nlh2cayaaewj0fvuaz39mcj7xf9g3wv33gyhv428kujckuqndct66",
            ),
            fee: Decimal::percent(10),
         },
      },
      game_delay: 300u64,
      admins: Some(vec![Addr::unchecked("kujira1y3ztnmghrmsa8d8h5ny7h2lvq4w3lre9hvwhcw")]),
   };

   // Serialize the InstantiateMsg instance to JSON
   let json_data = to_json_string(&msg).unwrap();

   // Print the JSON representation
   println!("Serialized JSON:\n{}", json_data);
}
