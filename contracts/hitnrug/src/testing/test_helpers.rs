use cosmwasm_std::{testing::MockApi, to_json_string, Addr, Coin, Decimal, Timestamp, Uint128};
use cw_multi_test::{App, AppResponse, BasicAppBuilder, ContractWrapper, Executor};

use crate::{
   config::{Config, ConfigUpdate},
   contract::{execute, instantiate, query},
   game::GameSmSnapshot,
   msg::{ExecuteMsg, Fee, Fees, InstantiateMsg, Point, Points, QueryMsg},
};

pub struct TestEnv {
   pub app: App,
   pub owner: Addr,
   pub contracts: MockContracts,
}

pub struct MockContracts {
   pub game: Addr,
   pub referral: Addr,
}

pub struct PartialInstantiate {
   pub owner: Addr,
   pub ticket_denom: String,
   pub ticket_amount: Uint128,
   pub starts_at: Timestamp,
   pub duration_seconds: u64,
   pub game_delay_sec: u64,
   pub delay_play_seconds: u64,
   pub fees: Vec<Decimal>,
   pub points: Points,
}

pub fn setup_test_env(balances: Vec<(Addr, Vec<Coin>)>, config: PartialInstantiate) -> TestEnv {
   let mut app = BasicAppBuilder::new().build(|router, _, storage| {
      for (addr, coins) in balances {
         router.bank.init_balance(storage, &addr, coins).unwrap();
      }
   });

   let ref_code_id = app.store_code(Box::new(ContractWrapper::new(
      referral::contract::execute,
      referral::contract::instantiate,
      referral::contract::query,
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
            game_delay_sec: config.game_delay_sec,
            duration_seconds: config.duration_seconds,
            delay_play_seconds: config.delay_play_seconds,
            fees: Fees {
               fee_platform: Fee { address: app.api().addr_make("owner"), bp: config.fees[0] },
               fee_ref: Fee { address: ref_addr.clone(), bp: config.fees[1] },
            },
            points: config.points,
         },
         &[],
         "game",
         None,
      )
      .unwrap();

   TestEnv {
      app,
      owner: MockApi::default().addr_make("owner"),
      contracts: MockContracts { game: game_addr, referral: ref_addr },
   }
}

pub fn create_partial_instantiate(
   owner: &str,
   ticket_denom: &str,
   ticket_amount: Uint128,
   starts_at: Timestamp,
   duration_seconds: u64,
   delay_play_seconds: u64,
   fees: Vec<Decimal>,
   pt_keep: i64,
   pt_hit: i64,
   pt_help: (i64, i64),
) -> PartialInstantiate {
   PartialInstantiate {
      owner: MockApi::default().addr_make(&owner),
      ticket_denom: ticket_denom.to_string(),
      ticket_amount,
      starts_at,
      duration_seconds,
      fees,
      delay_play_seconds,
      points: Points {
         keep: pt_keep,
         hit: pt_hit,
         help: Point { myself: pt_help.0, other: pt_help.1 },
      },
      game_delay_sec: 10u64,
   }
}

impl TestEnv {
   pub fn addr(&self, account: &str) -> Addr {
      self.app.api().addr_make(account)
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

   pub fn endgame(&mut self, account: &str) -> anyhow::Result<AppResponse> {
      self.app.execute_contract(
         self.addr(account),
         self.contracts.game.clone(),
         &ExecuteMsg::EndGame {},
         &[],
      )
   }

   pub fn play_keep(&mut self, account: &str) -> anyhow::Result<AppResponse> {
      self.app.execute_contract(
         self.addr(account),
         self.contracts.game.clone(),
         &ExecuteMsg::Play(crate::msg::PlayMsg::Keep {}),
         &[],
      )
   }
   pub fn play_hit(&mut self, account: &str, target: &str) -> anyhow::Result<AppResponse> {
      self.app.execute_contract(
         self.addr(account),
         self.contracts.game.clone(),
         &ExecuteMsg::Play(crate::msg::PlayMsg::Hit { target: self.addr(target) }),
         &[],
      )
   }

   pub fn play_help(&mut self, account: &str, target: &str) -> anyhow::Result<AppResponse> {
      self.app.execute_contract(
         self.addr(account),
         self.contracts.game.clone(),
         &ExecuteMsg::Play(crate::msg::PlayMsg::Help { target: self.addr(target) }),
         &[],
      )
   }

   pub fn update_config(
      &mut self,
      account: &str,
      new_config: ConfigUpdate,
   ) -> anyhow::Result<AppResponse> {
      self.app.execute_contract(
         self.addr(account),
         self.contracts.game.clone(),
         &ExecuteMsg::UpdateConfig { new_config },
         &[],
      )
   }

   pub fn get_snap(&mut self, idx: Option<u64>) -> GameSmSnapshot {
      self
         .app
         .wrap()
         .query_wasm_smart::<GameSmSnapshot>(
            self.contracts.game.clone(),
            &QueryMsg::GameStatus { idx },
         )
         .unwrap()
   }

   pub fn get_config(&mut self) -> Config {
      self
         .app
         .wrap()
         .query_wasm_smart::<Config>(self.contracts.game.clone(), &QueryMsg::Config {})
         .unwrap()
   }

   pub fn verify_winner(&mut self, account: &str) {
      let query: GameSmSnapshot = self
         .app
         .wrap()
         .query_wasm_smart(self.contracts.game.clone(), &QueryMsg::GameStatus { idx: None })
         .unwrap();

      assert_eq!(self.addr(account), query.current_winner.unwrap().0)
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
fn test_instantiate_msg_json() {
   let instance = InstantiateMsg {
      owner: Addr::unchecked("kujira15m5jv9ttlkvchkaca72wse7v8zx7hll4x6u0cf"),
      ticket_denom: "ukuji".to_string(),
      ticket_amount: Uint128::new(100000),
      starts_at: Timestamp::from_seconds(1730851800),
      duration_seconds: 300u64,
      game_delay_sec: 60u64,
      delay_play_seconds: 35u64,
      fees: Fees {
         fee_platform: Fee {
            address: Addr::unchecked("kujira15m5jv9ttlkvchkaca72wse7v8zx7hll4x6u0cf"),
            bp: Decimal::bps(1000),
         },
         fee_ref: Fee {
            address: Addr::unchecked(
               "kujira1rxud2nlh2cayaaewj0fvuaz39mcj7xf9g3wv33gyhv428kujckuqndct66",
            ),
            bp: Decimal::bps(1000),
         },
      },
      points: Points { keep: 4i64, hit: -5i64, help: Point { myself: 6, other: 4 } },
   };

   // Serialize the instance to JSON and print it
   let json_instance = to_json_string(&instance).unwrap();
   println!("{}", json_instance);
}
