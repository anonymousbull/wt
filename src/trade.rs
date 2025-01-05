use crate::constant::{RAYDIUM_V4_AUTHORITY, RAYDIUM_V4_PROGRAM, SOLANA_MINT, SOLANA_MINT_STR};
use crate::position::PositionConfig;
use crate::program_log::{ProgramLog, ProgramLogInfo};
use crate::rpc::{rpc1, Rpc, RpcResponse, RpcResponseData, RpcResponseMetric, RpcType, TradeRpcLog, TradeRpcLogGeneral, TradeRpcLogJito, TradeRpcLogStatus};
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
use anyhow::{anyhow, Error};
use base64::Engine;
use futures::stream::FuturesUnordered;
use log::{error, info};
use serde_json::json;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::commitment_config::CommitmentLevel;
use tokio::task::JoinHandle;
use crate::pump;
use crate::pump::{PumpTrade, PumpTradeData};
use crate::solana::*;

#[derive(Clone, Debug,Serialize,Deserialize)]
pub struct Trade {
    pub id: i64,
    pub decimals: i16,
    pub token_program_id: String,

    pub amount: Decimal,
    pub buy_time: Option<DateTime<Utc>>,
    pub buy_price: Option<Decimal>,
    pub sell_time: Option<DateTime<Utc>>,
    pub sell_price: Option<Decimal>,

    pub pct: Decimal,
    pub state: TradeState,

    pub sol_before: Decimal,
    pub sol_after: Option<Decimal>,

    pub root_kp:Vec<u8>,
    pub amm: TradePool,
    pub user_wallet: String,

    pub attempts: usize,
    pub rpc_logs:Vec<TradeRpcLog>,

    pub price:Decimal,
    pub k: Decimal,
    pub tvl:Decimal,

    pub instructions: Vec<TradeInstruction>,
    pub cfg: PositionConfig,
    pub transactions: Vec<TradeTransaction>,
    pub error: Option<String>,

    pub plog:ProgramLog,

    pub buy_out: Option<u64>,
}

#[derive(Clone,Serialize,Deserialize,Debug)]
pub enum TradeInstruction {
    Jito(Vec<Instruction>),
    General(Vec<Instruction>)
}

#[derive(Clone,Serialize,Deserialize,Debug, PartialEq, Eq)]
pub enum TradeSignature {
    PendingBuy(String),
    PendingSell(String),
    BuySuccess(String),
    SellSuccess(String),
}



#[derive(Clone,Serialize,Deserialize,Debug, PartialEq)]
pub enum TradeTransaction {
    Jito(Transaction),
    General(Transaction)
}

impl TradeTransaction {
    pub fn transaction(&self) ->&Transaction{
        match self {
            TradeTransaction::Jito(v) => v,
            TradeTransaction::General(v) => v
        }
    }
}

impl TradeInstruction {
    pub fn instructions(&self)->&Vec<Instruction>{
        match self {
            TradeInstruction::Jito(v) => v,
            TradeInstruction::General(v) => v
        }
    }
}

#[derive(Clone,Copy,Serialize,Deserialize,Debug)]
pub struct PumpBondingCurve{
    pub amm:Pubkey,
    pub vault:Pubkey,
    pub mint:Pubkey
}

#[derive(Clone,Copy,Serialize,Deserialize,Debug)]
pub struct RayAmm4{
    pub amm:Pubkey,
    pub coin_vault: Pubkey,
    pub pc_vault: Pubkey,
    pub coin_mint: Pubkey,
    pub pc_mint: Pubkey,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum TradePool {
    RayAmm4(RayAmm4),
    PumpBondingCurve(PumpBondingCurve),
}

impl TradePool {
    pub fn to_string(&self)->String{
        match self {
            TradePool::RayAmm4(v) => v.amm.to_string(),
            TradePool::PumpBondingCurve(v) => v.amm.to_string(),
        }
    }
    pub fn amm(&self) ->Pubkey{
        match self {
            TradePool::RayAmm4(v) => v.amm,
            TradePool::PumpBondingCurve(v) => v.amm,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
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
    #[default]
    Buy = 1,
    PendingBuy,
    BuySuccess,
    PendingSell,
    SellSuccess,
    BuyFailed,
    SellFailed
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



pub struct NewTrade {
    pub amm:TradePool,
    pub token_program_id: Pubkey,
    pub user_wallet: Pubkey,
    pub cfg: PositionConfig,
    pub log: String,
}

impl Trade {
    pub fn try_new(
        new_trade: NewTrade
    )->anyhow::Result<Self>{
        let NewTrade{amm,token_program_id,user_wallet,cfg, log} = new_trade;
        Self {
            id: 0,
            decimals: 0,
            token_program_id:token_program_id.to_string(),
            user_wallet: user_wallet.to_string(),
            amm,
            cfg,
            amount: Default::default(),
            buy_time: None,
            buy_price: None,
            sell_time: None,
            sell_price: None,
            pct: Default::default(),
            state: Default::default(),
            sol_before: Default::default(),
            sol_after: None,
            root_kp: vec![],
            attempts: 0,
            rpc_logs: vec![],
            price: Default::default(),
            k: Default::default(),
            tvl: Default::default(),
            instructions: vec![],
            transactions: vec![],
            error: None,
            plog: Default::default(),
            buy_out: None,
        }.update_price_from_log(log, true)
    }
    pub fn console_log(&self) -> String {
        let mut json = json!({});
        let success_signature = self.rpc_logs.iter().find_map(|x|
            match x {
                TradeRpcLog::SellJito(v, TradeRpcLogStatus::Success) => Some(v.general.signature.to_string()),
                TradeRpcLog::SellGeneral(v, TradeRpcLogStatus::Success) => Some(v.signature.to_string()),
                TradeRpcLog::BuyJito(v, TradeRpcLogStatus::Success) => Some(v.general.signature.to_string()),
                TradeRpcLog::BuyGeneral(v, TradeRpcLogStatus::Success) => Some(v.signature.to_string()),
                _ => None
            }
        );
        let jito_tip = self
            .rpc_logs
            .iter()
            .find_map(|x| match x {
                TradeRpcLog::BuyJito(v,_) => Some(v.tip_sol_ui.to_string()),
                TradeRpcLog::SellJito(v,_) => Some(v.tip_sol_ui.to_string()),
                _ => None,
            });

        json["state"] = json!(self.state);
        json["success_signature"] = json!(success_signature);
        json["id"] = json!(self.id);
        json["pct"] = json!(self.pct);
        json["price"] = json!(self.price);
        json["mint"] = json!(self.mint().to_string());
        json["jito_tip"] = json!(jito_tip);
        json["error"] = json!(self.error);
        json.to_string()
    }
    pub fn tx_id_unchecked(&self) ->Option<Signature>{
        self.rpc_logs.iter().find_map(|x|Some(x.signature()))

    }
    pub fn jito_fee_sol_ui(&self) ->Decimal {
        (self.cfg.jito_tip_percent / dec!(2)) * self.cfg.fee()
    }
    pub fn jito_fee_sol(&self) ->Decimal {
        let jito_fee_sol = self.jito_fee_sol_ui() * lamports_per_sol_dec();
        jito_fee_sol
    }
    pub fn jito_pfee_micro_sol(&self) -> Decimal {
        let pfee_sol_normal = (self.cfg.jito_priority_percent / dec!(2)) * self.cfg.fee();
        let pfee_micro_sol = pfee_sol_normal * micro_lamports_per_sol_dec();
        pfee_micro_sol
    }
    pub async fn build_transactions(mut self) ->Self{
        let hash = if let Rpc::General {rpc:read_rpc,..} = rpc1() {
            read_rpc.get_latest_blockhash().await.unwrap()
        } else {
            unreachable!()
        };
        let txs = self.instructions.iter().map(|x|{
            match x {
                TradeInstruction::Jito(v) => {
                    TradeTransaction::Jito(
                        Transaction::new_signed_with_payer(
                            &x.instructions().as_slice(),
                            Some(&self.root_kp().pubkey()),
                            &[self.root_kp()],
                            hash,
                        )
                    )
                }
                TradeInstruction::General(v) => {
                    TradeTransaction::General(
                        Transaction::new_signed_with_payer(
                            &x.instructions().as_slice(),
                            Some(&self.root_kp().pubkey()),
                            &[self.root_kp()],
                            hash,
                        )
                    )
                }
            }
        }).collect::<Vec<_>>();
        self.transactions = txs;
        self
    }
    pub fn is_buy(&self)->bool{
        match self.state {
            TradeState::Buy|TradeState::PendingBuy => true,
            _ => false,
        }
    }

    pub fn to_trade_price(&self,price_id:i64)->TradePrice{
        TradePrice{
            id: price_id,
            trade_id: self.id,
            price: self.price,
            tvl: self.tvl,
        }
    }

    pub fn update_price_from_log(mut self, log:String, next:bool) -> anyhow::Result<Self> {
        let plog = ProgramLog::from(log.clone())?;
        let (pc,coin) = match &plog.log {
            ProgramLogInfo::RayInitLog(init) => {
                self.decimals = if init.pc_decimals == 9 {
                    init.coin_decimals
                } else {
                    init.pc_decimals
                } as i16;
                (init.pc_amount, init.coin_amount)
            }
            ProgramLogInfo::RaySwapBaseIn(swap) => {
                if next {
                    (plog.next_pc, plog.next_coin)
                } else {
                    (swap.pool_pc, swap.pool_coin)
                }
            }
            ProgramLogInfo::RaySwapBaseOut(swap) => {
                if next {
                    (plog.next_pc, plog.next_coin)
                } else {
                    (swap.pool_pc, swap.pool_coin)
                }
            }
            ProgramLogInfo::PumpTradeLog(c) => {
                self.decimals = 6;
                (plog.next_pc, plog.next_coin)
            }
            _ => {
                return Err(anyhow!("known program, unknown event\nlog {log}\nplog: {plog:?}"));
            },
        };

        let sol_amount = min(pc, coin);
        let token_amount = max(pc, coin);

        // 1000
        let token_amount_ui = Decimal::from(token_amount) / dec!(10).powd(Decimal::from(self.decimals));
        // 1
        let sol_amount_ui = Decimal::from(sol_amount) / Decimal::from(LAMPORTS_PER_SOL);
        // 1 MEMECOIN = 0.0001 SOL
        let price = sol_amount_ui / token_amount_ui;
        let tvl = (price * token_amount_ui) + sol_amount_ui;
        let k = sol_amount_ui * token_amount_ui;

        self.price = price;
        self.tvl = tvl;
        self.k = k;
        self.plog = plog;
        Ok(self)
    }
    pub fn sol_mint(&self) -> Pubkey {
        match self.amm {
            TradePool::RayAmm4(amm) => {
                if amm.coin_mint == SOLANA_MINT {
                    amm.coin_mint
                } else if amm.pc_mint == SOLANA_MINT {
                    amm.pc_mint
                } else {
                    unreachable!()
                }
            }
            TradePool::PumpBondingCurve(_) => SOLANA_MINT
        }
    }
    pub fn mint(&self) -> Pubkey {
        match self.amm {
            TradePool::RayAmm4(amm) => {
                if amm.coin_mint == SOLANA_MINT {
                    amm.pc_mint
                } else if amm.pc_mint == SOLANA_MINT {
                    amm.coin_mint
                } else {
                    unreachable!()
                }
            }
            TradePool::PumpBondingCurve(amm) => {
                amm.mint
            }
        }

    }
    pub fn root_kp(&self) -> Keypair {
        Keypair::from_bytes(&self.root_kp).unwrap()
    }
    pub fn token_program(&self)->Pubkey{
        Pubkey::from_str(&self.token_program_id.as_str()).unwrap()
    }
    pub fn amm(&self)->Pubkey{
        self.amm.amm()
    }
    pub fn coin_mint(&self)->Pubkey{
        match self.amm {
            TradePool::RayAmm4(amm) => {
                amm.coin_mint
            }
            TradePool::PumpBondingCurve(_) => {
                unreachable!()
            }
        }
    }
    pub fn pc_mint(&self)->Pubkey{
        match self.amm {
            TradePool::RayAmm4(amm) => {
                amm.pc_mint
            }
            TradePool::PumpBondingCurve(_) => {
                unreachable!()
            }
        }
    }
    pub fn coin_vault(&self)->Pubkey{
        match self.amm {
            TradePool::RayAmm4(amm) => {
                amm.coin_vault
            }
            TradePool::PumpBondingCurve(_) => {
                unreachable!()
            }
        }
    }
    pub fn pc_vault(&self)->Pubkey{
        match self.amm {
            TradePool::RayAmm4(amm) => {
                amm.pc_vault
            }
            TradePool::PumpBondingCurve(_) => {
                unreachable!()
            }
        }
    }
    pub fn is_sol_pool(&self) -> bool {
        match self.amm {
            TradePool::RayAmm4(amm) => {
                amm.pc_mint == SOLANA_MINT || amm.coin_mint == SOLANA_MINT
            }
            TradePool::PumpBondingCurve(_) => {
                unreachable!()
            }
        }
    }
    pub async fn send_many(mut self, rpcs: Vec<Rpc>)->FuturesUnordered<JoinHandle<Result<Trade,Trade>>>{
        let futs= FuturesUnordered::new();
        let trade = self.build_transactions().await;
        for rpc in rpcs {
            futs.push(tokio::spawn({
                let trade = trade.clone();
                async move {
                    trade.send(rpc).await
                }
            }));
        }
        futs
    }
    pub async fn send(mut self, rpc:Rpc) -> Result<Trade,Trade> {
        let start_time = ::std::time::Instant::now();
        let mut log_info = None;

        let res = match rpc {
            Rpc::Jito { rpc,info,.. } => {
                let mut trade = self.clone();

                let tx = trade.transactions.iter()
                    .find(|&x|matches!(x, TradeTransaction::Jito(_)))
                    .unwrap().transaction().clone();
                let ix = trade.instructions.iter()
                    .find(|x|matches!(x, TradeInstruction::Jito(_)))
                    .unwrap().instructions().clone();
                let serialized_tx =
                    base64::engine::general_purpose::STANDARD.encode(bincode::serialize(&tx).unwrap());
                let params = json!({
                    "tx": serialized_tx,
                    "skipPreflight": true
                });
                if self.is_buy() == false {
                    if let Rpc::General {rpc,..} = rpc1() {
                        info!("what is going on");
                        rpc.send_transaction_with_config(&tx,RpcSendTransactionConfig{
                            skip_preflight: true,
                            preflight_commitment: Some(CommitmentLevel::Processed),
                            encoding: None,
                            max_retries: None,
                            min_context_slot: None,
                        }).await.unwrap();
                        panic!("go go");
                    }
                }

                let resp = rpc.send_txn(Some(params), true).await.unwrap();
                let sig = resp["result"]
                    .as_str()
                    .ok_or_else(|| anyhow::format_err!("Failed to get signature from response"));
                log_info = Some(info.clone());
                info!("{} {} Trade sent tx: {:?}",info.name, info.location, start_time.elapsed());
                let t = self.clone();
                sig.inspect(|x|{
                    info!("https://solscan.io/tx/{x}");
                    info!("https://pump.fun/coin/{}",t.mint().to_string());

                }).map(|s| {
                    let general = TradeRpcLogGeneral{
                        signature: s.to_string(),
                        response_time: format!("{:?}", start_time.elapsed()),
                        name: format!("{}",info.name),
                        tx: tx.clone(),
                        ix: ix.clone()
                    };
                    let log = TradeRpcLogJito{
                        general,
                        tip_sol_ui: trade.jito_fee_sol_ui().to_string(),
                    };
                    if trade.is_buy() {
                        trade.state = TradeState::PendingBuy;
                        trade.rpc_logs.push(TradeRpcLog::BuyJito(log,TradeRpcLogStatus::Pending));
                    } else {
                        trade.state = TradeState::PendingSell;
                        trade.rpc_logs.push(TradeRpcLog::SellJito(log,TradeRpcLogStatus::Pending));
                    }
                    trade
                })
            }
            Rpc::General { rpc,info } => {
                let mut trade = self.clone();

                let tx = trade.transactions.iter()
                    .find(|x|matches!(x, TradeTransaction::General(_)))
                    .unwrap().transaction().clone();
                let ix = trade.instructions.iter()
                    .find(|x|matches!(x, TradeInstruction::General(_)))
                    .unwrap().instructions().clone();

                let resp = rpc
                    .send_transaction_with_config(
                        &tx,
                        solana_client::rpc_config::RpcSendTransactionConfig {
                            skip_preflight: false,
                            preflight_commitment: Some(solana_sdk::commitment_config::CommitmentLevel::Processed),
                            encoding: None,
                            max_retries: None,
                            min_context_slot: None,
                        }
                        // &rpc_client.get_latest_blockhash().await.unwrap(),
                    )
                    .await
                    .map(|x|{
                    let log = TradeRpcLogGeneral{
                        signature: x.to_string(),
                        response_time: format!("{:?}", start_time.elapsed()),
                        name: format!("{}",info.name),
                        tx: tx.clone(),
                        ix: ix.clone()
                    };
                    if trade.is_buy() {
                        trade.state = TradeState::PendingBuy;
                        trade.rpc_logs.push(TradeRpcLog::BuyGeneral(log,TradeRpcLogStatus::Pending));
                    } else {
                        trade.state = TradeState::PendingSell;
                        trade.rpc_logs.push(TradeRpcLog::SellGeneral(log,TradeRpcLogStatus::Pending));
                    }
                    trade
                })
                    .map_err(anyhow::Error::from);
                resp
            }
        }
            .map(|mut x|{
                x.transactions = Default::default();
                x.instructions = Default::default();
                x
            })
            .map_err(|x|{
                self.transactions = Default::default();
                self.instructions = Default::default();
            self.error = Some(x.to_string());
            if self.is_buy() {
                self.state = TradeState::BuyFailed;
            } else {
                self.state = TradeState::SellFailed;
            }
            self.clone()
        });
        info!("Trade sent tx: {:?}", start_time.elapsed());
        res
    }
    pub fn build_instructions(mut self) -> Self {
        let one = dec!(1);
        let gas_limit = match self.amm {
            TradePool::RayAmm4(_) => {
                if self.is_buy() {
                    self.cfg.ray_buy_gas_limit
                } else {
                    self.cfg.ray_sell_gas_limit
                }
            }
            TradePool::PumpBondingCurve(_) => {
            if self.is_buy() {
                    self.cfg.pump_buy_gas_limit
                } else {
                    self.cfg.pump_sell_gas_limit
                }
            }
        };

        assert!(gas_limit.to_u64().unwrap() > 0);
        let cfg = self.cfg;
        let token_program_id = self.token_program();
        let kp = self.root_kp();
        let pubkey = kp.pubkey();
        let amm = self.amm();
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
        // 1. https://solscan.io/tx/5ERUd3HpkQwhBNoTi82YVWFTdAz4Y5Xhjst69LFSiARFdxiGJtZELAPJtrdZQqgkjDnXbb2zVBnGCnqCT9Z5G5yn
        // slippage higher than amount
        // 2. https://solscan.io/tx/4eu6cNN8b8EtDSYcaqco1BbEirVcEDep1TTnd2v2cAJXKsKC6VCWw4mavC3YN2MoFyPvpMMCQL5XcuQ36W1wQafB
        // amount higher than slippage
        // 1. got exact price
        let decimals = self.decimals;
        let max_in_token_ui = cfg.max_sol / self.price;
        let max_in_sol = cfg.max_sol * lamports_per_sol_dec();

        let max_in_token = max_in_token_ui * dec!(10).powd(Decimal::from(decimals));


        let min_out_token = max_in_token * (dec!(1)-cfg.slippage);

        info!("min_out_token {min_out_token}");
        info!("max_in_token_ui {max_in_token_ui}");
        info!("max_in_sol {max_in_sol}");
        info!("max_in_token {max_in_token}");
        info!("max_sol {}", cfg.max_sol);
        info!("price {}", self.price);

        let mut instructions = vec![
            if self.is_buy() {
                ComputeBudgetInstruction::set_compute_unit_limit(gas_limit as u32)
            } else {
                ComputeBudgetInstruction::set_compute_unit_limit(gas_limit as u32)
            },
            // IT WILL TAKE FEES HERE
            // https://solscan.io/tx/27axTimhcskhx8gnphFtu6LssVkV8qbzgaDNm6v1SNagZhPiY3MzDzDrYqPTdfC5gdUhvJVimBpueQ2ygj9wpaS3
            spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                &pubkey,
                &pubkey,
                &self.mint(),
                &token_program_id,
            )
        ];



        match self.amm {
            TradePool::RayAmm4(_) => {
                if self.is_buy() {
                    // let amount_normal = cfg.min_sol * Decimal::from(LAMPORTS_PER_SOL);
                    // let slippage_normal = (cfg.max_sol / price) * dec!(10).powd(Decimal::from(decimals));
                    instructions.extend_from_slice(&vec![
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
                            max_in_token.to_u64().unwrap(),
                            min_out_token.to_u64().unwrap(),
                        )
                            .unwrap(),
                    ]);
                } else {
                    instructions.extend_from_slice(&vec![
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
                }
            }
            TradePool::PumpBondingCurve(curve) => {
                let min = if self.is_buy() {
                    min_out_token.to_u64().unwrap()
                } else {
                    Default::default()
                };
                let max = if self.is_buy() {
                    max_in_sol.to_u64().unwrap()
                } else {
                    self.buy_out.unwrap()
                };
                let pump_ix = pump::trade(
                    PumpTrade {
                        data: PumpTradeData {
                            min,
                            max,
                        },
                        mint: self.mint(),
                        curve:curve.amm,
                        ata_curve: curve.vault,
                        ata_user: token,
                        user: self.root_kp().pubkey(),
                    },
                    self.is_buy()
                ).unwrap();
                instructions.push(pump_ix);
            }
        }


        vec![RpcType::Jito,RpcType::General].iter().for_each(|x|{
            match x {
                RpcType::Jito => {
                    let jito_fee_sol_normal = (self.cfg.jito_tip_percent / dec!(2)) * self.cfg.fee();
                    let jito_fee_sol = jito_fee_sol_normal * lamports_per_sol_dec();

                    let pfee_sol_normal = (self.cfg.jito_priority_percent / dec!(2)) * self.cfg.fee();
                    let pfee_micro_sol = pfee_sol_normal * micro_lamports_per_sol_dec();


                    let gas_price = pfee_micro_sol / Decimal::from(gas_limit);
                    info!("max fees {}",self.cfg.tp());
                    info!("jito fee {}",jito_fee_sol_normal);
                    info!("pfee fee {}",pfee_sol_normal);
                    info!("pfee_micro_sol {}",pfee_micro_sol);
                    info!("gas limit {}",gas_limit);
                    info!("gas price {}",gas_price);
                    info!("jito ix value {}",jito_fee_sol.to_u64().unwrap());
                    let jito_instructions = vec![
                        &vec![
                            ComputeBudgetInstruction::set_compute_unit_price(gas_price.to_u64().unwrap())
                        ][..],
                        &instructions[..],
                        &vec![
                            solana_sdk::system_instruction::transfer(
                                &self.root_kp().pubkey(),
                                &Pubkey::from_str_const(crate::constant::JITO_TIPS[fastrand::usize(..crate::constant::JITO_TIPS.len())]),
                                jito_fee_sol.to_u64().unwrap(),
                            ),
                        ][..]
                    ].concat();
                    self.instructions.push(TradeInstruction::Jito(jito_instructions));
                }
                RpcType::General => {
                    self.instructions.push(TradeInstruction::General(instructions.clone()));
                }
            }
        });
        self
    }

    pub fn from_raydium_init(accounts: &[Pubkey], cfg: PositionConfig, log: String) -> anyhow::Result<Self> {
        let token_program_id = accounts[0].to_string();
        let amm = accounts[4].to_string();
        let coin_mint = accounts[8].to_string();
        let pc_mint = accounts[9].to_string();
        let coin_vault = accounts[10].to_string();
        let pc_vault = accounts[11].to_string();
        let user_wallet = accounts[17].to_string();
        Self::try_new(
            NewTrade{
                log,
                cfg,
                amm:TradePool::RayAmm4(RayAmm4{
                    amm:Pubkey::from_str_const(amm.as_str()),
                    coin_vault:Pubkey::from_str_const(coin_vault.as_str()),
                    pc_vault:Pubkey::from_str_const(pc_vault.as_str()),
                    coin_mint:Pubkey::from_str_const(coin_mint.as_str()),
                    pc_mint:Pubkey::from_str_const(pc_mint.as_str()),
                }),
                token_program_id:Pubkey::from_str_const(token_program_id.as_str()),
                user_wallet:Pubkey::from_str_const(user_wallet.as_str()),
            }
        )
    }

    pub fn from_pump_swap(accounts: &[Pubkey], cfg: PositionConfig, log: String) -> anyhow::Result<Self> {
        let token_program_id = accounts[8].to_string();
        let amm = accounts[3].to_string();
        let mint = accounts[2].to_string();
        let pc_mint = SOLANA_MINT_STR.to_string();
        let vault = accounts[4].to_string();
        let pc_vault = accounts[11].to_string();
        let user_wallet = accounts[6].to_string();
        Self::try_new(
            NewTrade{
                log,
                amm:TradePool::PumpBondingCurve(PumpBondingCurve{
                    amm:Pubkey::from_str_const(amm.as_str()),
                    vault:Pubkey::from_str_const(vault.as_str()),
                    mint:Pubkey::from_str_const(mint.as_str()),
                }),
                token_program_id:Pubkey::from_str_const(token_program_id.as_str()),
                user_wallet:Pubkey::from_str_const(user_wallet.as_str()),
                cfg,
            }
        )
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




