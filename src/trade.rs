use crate::constant::{RAYDIUM_V4_AUTHORITY, RAYDIUM_V4_PROGRAM, SOLANA_MINT_STR};
use crate::position::PositionConfig;
use crate::ray_log::{RayLog, RayLogInfo};
use crate::rpc::{rpc1, Rpc, RpcResponse, RpcResponseData, RpcResponseMetric};
use chrono::{DateTime, Utc};
use raydium_amm::solana_program::native_token::{sol_to_lamports, LAMPORTS_PER_SOL};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::{Decimal, MathematicalOps};
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signature};
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use spl_associated_token_account_client::address::get_associated_token_address_with_program_id;
use std::cmp::{max, min};
use std::str::FromStr;
use anyhow::Error;
use base64::Engine;
use log::info;
use serde_json::json;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::commitment_config::CommitmentLevel;
use crate::solana::*;

#[derive(Clone, Debug,Serialize,Deserialize,Default)]
pub struct Trade {
    pub id: i64,
    pub coin_vault: String,
    pub pc_vault: String,
    pub coin_mint: String,
    pub pc_mint: String,
    pub decimals: i16,
    pub token_program_id: String,

    pub amount: Decimal,
    pub entry_time: Option<DateTime<Utc>>,
    pub entry_price: Option<Decimal>,
    pub exit_time: Option<DateTime<Utc>>,
    pub exit_price: Option<Decimal>,

    pub pct: Decimal,
    pub state: TradeState,

    pub buy_id:Option<String>,
    pub sell_id:Option<String>,
    pub sol_before: Decimal,
    pub sol_after: Option<Decimal>,

    pub root_kp:Vec<u8>,
    pub pool_id: String,
    // pub pump_curve_id: String,
    pub user_wallet: String,

    pub buy_ids:Vec<String>,
    pub sell_ids:Vec<String>,
    pub buy_attempts: usize,
    pub sell_attempts: usize,

    pub buy_logs: Vec<RpcResponse>,
    pub sell_logs: Vec<RpcResponse>,
    pub buy_log: Option<RpcResponse>,
    pub sell_log: Option<RpcResponse>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TradePrice {
    pub id: i64,
    pub trade_id: i64,
    pub price: Decimal,
    pub tvl: Decimal,
}


#[derive(Clone)]
pub struct TradeRequest {
    pub trade: Trade,
    pub instructions: Vec<Instruction>,
    pub config: PositionConfig,
    pub gas_limit: Decimal,

}

#[derive(Debug)]
pub struct TradeResponse {
    pub instructions: Vec<Instruction>,
    pub trade: Trade,
    pub sig: String,
    pub rpc_response: RpcResponse,
}

pub type TradeResponseResult = Result<TradeResponse, TradeResponseError>;

#[repr(i32)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash, Default)]
pub enum TradeState {
    Init = 1,
    PositionRequest = 2,
    PendingBuy = 3,
    BuySuccess = 4,
    PendingSell = 5,
    PositionClosed = 6,
    BuyFailed,
    SellFailed,
    #[default]
    Empty
}


struct PriceTvl {
    tvl: Decimal,
    price: Decimal,
}

#[derive(Debug)]
pub struct TradeResponseError {
    pub trade: Trade,
    pub error: Error,
}

impl std::fmt::Display for TradeResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "TradeResponseError: {}", self.error)
    }
}

impl TradeRequest {
    pub async fn build_tx_and_send(self, rpc:Rpc) -> TradeResponseResult {
       let lamports_per_sol_dec = lamports_per_sol_dec();
       let micro_lamports_per_sol_dec = micro_lamports_per_sol_dec();

        let hash = if let Rpc::General {rpc:read_rpc,..} = rpc1() {
            read_rpc.get_latest_blockhash().await.unwrap()
        } else {
            unreachable!()
        };
        let one = dec!(1);

        match rpc {
            Rpc::Jito { rpc,info,.. } => {
                // The priority fee should equal our fee split
                // and it should be higher than average

                let jito_fee_sol_normal = (self.config.jito_tip_percent / dec!(2)) * self.config.fee();
                let jito_fee_sol = jito_fee_sol_normal * lamports_per_sol_dec;

                let pfee_sol_normal = (self.config.jito_priority_percent / dec!(2)) * self.config.fee();
                let pfee_micro_sol = pfee_sol_normal * micro_lamports_per_sol_dec;


                let gas_price = pfee_micro_sol / self.gas_limit;
                info!("max fees {}",self.config.tp());
                info!("jito fee {}",jito_fee_sol_normal);
                info!("pfee fee {}",pfee_sol_normal);
                info!("pfee_micro_sol {}",pfee_micro_sol);
                info!("gas limit {}",self.gas_limit);
                info!("gas price {}",gas_price);
                info!("jito ix value {}",jito_fee_sol.to_u64().unwrap());
                let jito_instructions = vec![
                    &vec![
                        ComputeBudgetInstruction::set_compute_unit_price(gas_price.to_u64().unwrap())
                    ][..],
                    &self.instructions[..],
                    &vec![
                        solana_sdk::system_instruction::transfer(
                            &self.trade.root_kp().pubkey(),
                            &Pubkey::from_str_const(crate::constant::JITO_TIPS[fastrand::usize(..crate::constant::JITO_TIPS.len())]),
                            jito_fee_sol.to_u64().unwrap(),
                        ),
                    ][..]
                ].concat();
                let tx = Transaction::new_signed_with_payer(
                    &jito_instructions,
                    Some(&self.trade.root_kp().pubkey()),
                    &[self.trade.root_kp()],
                    hash,
                );

                let start_time = ::std::time::Instant::now();
                let serialized_tx =
                    base64::engine::general_purpose::STANDARD.encode(bincode::serialize(&tx).unwrap());

                let params = json!({
                    "tx": serialized_tx,
                    "skipPreflight": true
                });


                // if let Rpc::General {rpc,..} = rpc1() {
                //     info!("what is going on");
                //     rpc.send_transaction_with_config(&tx,RpcSendTransactionConfig{
                //         skip_preflight: false,
                //         preflight_commitment: Some(CommitmentLevel::Processed),
                //         encoding: None,
                //         max_retries: None,
                //         min_context_slot: None,
                //     }).await.unwrap();
                //     panic!("go go");
                // }

                let resp = rpc.send_txn(Some(params), true).await.unwrap();
                let sig = resp["result"]
                    .as_str()
                    .ok_or_else(|| anyhow::format_err!("Failed to get signature from response"));

                info!("{} {} Trade sent tx: {:?}",info.name, info.location, start_time.elapsed());
                sig.map(|s| TradeResponse{
                    rpc_response:RpcResponse{
                        signature: s.to_string(),
                        metric: RpcResponseMetric {
                            response_time: start_time.elapsed().as_millis(),
                            response_time_string: format!("{:?}", start_time.elapsed()),
                        },
                        rpc_info: info.clone(),
                        rpc_response_data: RpcResponseData::Jito {
                            tip_amount_sol: jito_fee_sol,
                            priority_amount_micro_sol: pfee_micro_sol,
                        },
                    },
                    trade: self.trade.clone(),
                    instructions: self.instructions,
                    sig:s.to_string(),
                })
                    .map_err(|error|TradeResponseError{
                        trade:self.trade,error
                    })
            }
            Rpc::General { rpc,info } => {
                let pfee_sol = (one-self.config.jito_priority_percent) * self.config.tp();

                let tx = Transaction::new_signed_with_payer(
                    &self.instructions,
                    Some(&self.trade.root_kp().pubkey()),
                    &[self.trade.root_kp()],
                    hash,
                );
                let start_time = ::std::time::Instant::now();
                let resp = rpc
                    .send_transaction_with_config(
                        &tx,
                        solana_client::rpc_config::RpcSendTransactionConfig {
                            skip_preflight: true,
                            preflight_commitment: Some(solana_sdk::commitment_config::CommitmentLevel::Processed),
                            encoding: None,
                            max_retries: None,
                            min_context_slot: None,
                        }
                        // &rpc_client.get_latest_blockhash().await.unwrap(),
                    )
                    .await.map(|x|TradeResponse{
                    rpc_response:RpcResponse{
                        signature: x.to_string(),
                        metric: RpcResponseMetric {
                            response_time: start_time.elapsed().as_millis(),
                            response_time_string: format!("{:?}", start_time.elapsed()),
                        },
                        rpc_info: info.clone(),
                        rpc_response_data: RpcResponseData::General {
                            priority_amount_normal: pfee_sol,
                        },
                    },
                    trade: self.trade.clone(),
                    instructions: self.instructions,
                    sig:x.to_string(),
                })
                    .map_err(|err| anyhow::Error::msg(err.to_string()))
                    .map_err(|error|TradeResponseError{
                        trade:self.trade,error
                    });
                info!("{} {} Trade sent tx: {:?}",info.name, info.location, start_time.elapsed());
                resp
            }
        }
    }
}

impl Trade {

    fn price_tvl(sol_amount: u64, token_amount: u64, token_decimals: u8) -> PriceTvl {
        // 1000
        let token_amount_normal =
            Decimal::from(token_amount) / Decimal::from(10u64.pow(token_decimals as u32));
        // 1
        let sol_amount_normal = Decimal::from(sol_amount) / Decimal::from(LAMPORTS_PER_SOL);
        // 1 MEMECOIN = 0.0001 SOL
        let price = sol_amount_normal / token_amount_normal;
        let tvl = (price * token_amount_normal) + sol_amount_normal;
        PriceTvl { price, tvl  }
    }
    pub fn update_from_ray_log(&mut self, ray_log: &RayLog, price_id:i64, next:bool) -> TradePrice {
        let (pc,coin) = match &ray_log.log {
            RayLogInfo::InitLog(init) => {
                self.decimals = if init.pc_decimals == 9 {
                    init.coin_decimals
                } else {
                    init.pc_decimals
                } as i16;
                (init.pc_amount, init.coin_amount)
            }
            RayLogInfo::SwapBaseIn(swap) => {
                if next {
                    (ray_log.next_pc,ray_log.next_coin)
                } else {
                    (swap.pool_pc, swap.pool_coin)
                }
            }
            RayLogInfo::SwapBaseOut(swap) => {
                if next {
                    (ray_log.next_pc,ray_log.next_coin)
                } else {
                    (swap.pool_pc, swap.pool_coin)
                }
            }
            _ => unreachable!(),
        };

        let sol_amount = min(pc, coin);
        let token_amount = max(pc, coin);

        let price_tvl = Self::price_tvl(
            sol_amount,
            token_amount,
            self.decimals as u8,
        );

        TradePrice{
            id: price_id,
            trade_id: self.id,
            price: price_tvl.price,
            tvl: price_tvl.tvl,
        }
    }
    pub fn sol_mint(&self) -> Pubkey {
        if self.coin_mint.as_str() == SOLANA_MINT_STR {
            Pubkey::from_str_const(self.coin_mint.as_str())
        } else if self.pc_mint.as_str() == SOLANA_MINT_STR {
            Pubkey::from_str_const(self.pc_mint.as_str())
        } else {
            unreachable!()
        }
    }
    pub fn mint(&self) -> Pubkey {
        if self.coin_mint.as_str() == SOLANA_MINT_STR {
            Pubkey::from_str_const(self.pc_mint.as_str())
        } else if self.pc_mint.as_str() == SOLANA_MINT_STR {
            Pubkey::from_str_const(self.coin_mint.as_str())
        } else {
            unreachable!()
        }
    }
    pub fn root_kp(&self) -> Keypair {
        Keypair::from_bytes(&self.root_kp).unwrap()
    }
    pub fn token_program(&self)->Pubkey{
        Pubkey::from_str(&self.token_program_id.as_str()).unwrap()
    }
    pub fn amm(&self)->Pubkey{
        Pubkey::from_str(&self.pool_id.as_str()).unwrap()
    }
    pub fn coin_mint(&self)->Pubkey{
        Pubkey::from_str(&self.coin_mint.as_str()).unwrap()
    }
    pub fn pc_mint(&self)->Pubkey{
        Pubkey::from_str(&self.pc_mint.as_str()).unwrap()
    }
    pub fn coin_vault(&self)->Pubkey{
        Pubkey::from_str(&self.coin_vault.as_str()).unwrap()
    }
    pub fn pc_vault(&self)->Pubkey{
        Pubkey::from_str(&self.pc_vault.as_str()).unwrap()
    }
    pub fn is_sol_pool(&self) -> bool {
        self.pc_mint.as_str() == SOLANA_MINT_STR || self.coin_mint.as_str() == SOLANA_MINT_STR
    }
    pub fn create_position(self, price: Decimal, cfg: PositionConfig, open:bool) -> TradeRequest {

        // let jito_acc = Pubkey::from_str_const("NextbLoCkVtMGcV47JzewQdvBpLqT9TxQFozQkN98pE");
        // let jito_acc = Pubkey::from_str_const("HWEoBxYs7ssKuudEjzjmpfJVX7Dvi7wescFsVx2L5yoY");
        let jito_acc = Pubkey::from_str_const(crate::constant::JITO_TIPS[fastrand::usize(..crate::constant::JITO_TIPS.len())]);

        let token_program_id = self.token_program();
        let kp = self.root_kp();
        let pubkey = kp.pubkey();
        // let coin_mint = self.coin_mint();
        // let pc_mint = self.pc_mint();

        let amm = self.amm();
        // let coin_tk = get_associated_token_address_with_program_id(
        //     &pubkey,
        //     &coin_mint,
        //     &token_program_id,
        // );
        // let pc_tk = get_associated_token_address_with_program_id(
        //     &pubkey,
        //     &pc_mint,
        //     &token_program_id,
        // );
        let wsol = get_associated_token_address_with_program_id(
            &pubkey,
            &self.sol_mint(),
            &token_program_id,
        );
        let token = get_associated_token_address_with_program_id(
            &pubkey,
            &self.mint(),
            &token_program_id,
        );

        let mut parent_root_ix = vec![

        ];

        if open {
            parent_root_ix.push(ComputeBudgetInstruction::set_compute_unit_limit(cfg.buy_gas_limit.to_u32().unwrap()));
            let mut create_token_accounts = vec![];
            let mut create_token22_accounts = vec![];
            let mut rest_ix = vec![];

            // 1. https://solscan.io/tx/5ERUd3HpkQwhBNoTi82YVWFTdAz4Y5Xhjst69LFSiARFdxiGJtZELAPJtrdZQqgkjDnXbb2zVBnGCnqCT9Z5G5yn
            // slippage higher than amount
            // 2. https://solscan.io/tx/4eu6cNN8b8EtDSYcaqco1BbEirVcEDep1TTnd2v2cAJXKsKC6VCWw4mavC3YN2MoFyPvpMMCQL5XcuQ36W1wQafB
            // amount higher than slippage
            // 1. got exact price
            let decimals = self.decimals;

            let max_in = cfg.max_sol / price;
            let max_in_normal = max_in * dec!(10).powd(Decimal::from(decimals));
            let min_out_normal = max_in_normal * (dec!(1)-cfg.slippage);


            // let amount_normal = cfg.min_sol * Decimal::from(LAMPORTS_PER_SOL);
            // let slippage_normal = (cfg.max_sol / price) * dec!(10).powd(Decimal::from(decimals));
            let ca = if token_program_id == spl_token::id() {
                &mut create_token_accounts
            } else {
                &mut create_token22_accounts
            };

            // IT WILL TAKE FEES HERE
            // https://solscan.io/tx/27axTimhcskhx8gnphFtu6LssVkV8qbzgaDNm6v1SNagZhPiY3MzDzDrYqPTdfC5gdUhvJVimBpueQ2ygj9wpaS3
            if ca.is_empty() {
                ca.extend_from_slice(&vec![
                    spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                        &pubkey,
                        &pubkey,
                        &self.mint(),
                        &token_program_id,
                    ),
                ]);
            }

            rest_ix.extend_from_slice(&vec![
                raydium_amm::instruction::swap_base_out(
                    &RAYDIUM_V4_PROGRAM,
                    &amm,
                    &RAYDIUM_V4_AUTHORITY,
                    &amm,
                    &self.coin_vault(),
                    &self.pc_vault(),
                    &amm,
                    &amm,
                    &amm,
                    &amm,
                    &amm,
                    &amm,
                    &amm,
                    &amm,
                    &wsol,
                    &token,
                    &pubkey,
                    max_in_normal.to_u64().unwrap(),
                    min_out_normal.to_u64().unwrap(),
                )
                    .unwrap(),
            ]);

            TradeRequest {
                config:cfg,
                trade: Trade {
                    amount: Default::default(),
                    entry_time: None,
                    entry_price: None,
                    exit_price: None,
                    exit_time: None,
                    pct: Default::default(),
                    state: TradeState::PositionRequest,
                    buy_id: None,
                    sol_before: cfg.total_normal(),
                    sol_after: None,
                    ..self.clone()
                },
                instructions: vec![
                    parent_root_ix,
                    create_token_accounts,
                    create_token22_accounts,
                    rest_ix,
                ]
                    .concat(),
                gas_limit: cfg.buy_gas_limit,
            }
        } else {
            parent_root_ix.push(ComputeBudgetInstruction::set_compute_unit_limit(cfg.sell_gas_limit.to_u32().unwrap()));
            parent_root_ix.extend_from_slice(&vec![
                raydium_amm::instruction::swap_base_in(
                    &RAYDIUM_V4_PROGRAM,
                    &amm,
                    &RAYDIUM_V4_AUTHORITY,
                    &amm,
                    &self.coin_vault(),
                    &self.pc_vault(),
                    &amm,
                    &amm,
                    &amm,
                    &amm,
                    &amm,
                    &amm,
                    &amm,
                    &amm,
                    &token,
                    &wsol,
                    &kp.pubkey(),
                    self.amount.to_u64().unwrap(),
                    Default::default() // slippage default 0
                )
                    .unwrap(),
                spl_token::instruction::close_account(
                    &token_program_id,
                    &token,
                    &kp.pubkey(),
                    &kp.pubkey(),
                    &[&kp.pubkey()],
                )
                    .unwrap(),
                // build_memo_instruction(),
            ]);
            TradeRequest{
                config:cfg,
                trade: Trade {
                    ..self
                },
                instructions:parent_root_ix,
                gas_limit: cfg.sell_gas_limit,
            }
        }
    }

    pub fn from_solana_account_grpc(accounts: &[Pubkey]) -> Self {
        let token_program_id = accounts[0].to_string();
        let amm = accounts[4].to_string();
        let coin_mint = accounts[8].to_string();
        let pc_mint = accounts[9].to_string();
        let coin_vault = accounts[10].to_string();
        let pc_vault = accounts[11].to_string();
        let user_wallet = accounts[17].to_string();
        Self {
            id:Default::default(),
            pool_id: amm,
            coin_vault,
            pc_vault,
            coin_mint,
            pc_mint,
            // VERY RISKY
            decimals: Default::default(),
            token_program_id,
            user_wallet,
            ..Default::default()
        }
    }
}


fn build_memo_instruction() -> Instruction {
    let trader_apimemo_program =
        Pubkey::from_str_const("HQ2UUt18uJqKaQFJhgV9zaTdQxUZjNrsKFgoEDquBkcx");
    let accounts = vec![AccountMeta::new(trader_apimemo_program, false)];
    let bx_memo_marker_msg = String::from("Powered by bloXroute Trader Api")
        .as_bytes()
        .to_vec();
    Instruction {
        program_id: trader_apimemo_program,
        accounts,
        data: bx_memo_marker_msg,
    }
}




