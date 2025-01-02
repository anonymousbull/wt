use crate::constant::{Sdb, RAYDIUM_V4_AUTHORITY, RAYDIUM_V4_PROGRAM, SOLANA_MINT_STR};
use crate::db::SdbImpl;
use crate::position::PositionConfig;
use crate::ray_log::{RayLog, RayLogInfo};
use crate::send_tx::{SendTx, SendTxConfig};
use chrono::{DateTime, Utc};
use raydium_amm::solana_program::native_token::{sol_to_lamports, LAMPORTS_PER_SOL};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::{Decimal, MathematicalOps};
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use spl_associated_token_account_client::address::get_associated_token_address_with_program_id;
use std::cmp::{max, min};
use std::str::FromStr;
use anyhow::Error;

#[derive(Clone, Debug, PartialEq, Eq, Hash,Serialize,Deserialize,Default)]
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
    pub buy_attempts: i64,
    pub sell_attempts: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TradePrice {
    pub id: i64,
    pub trade_id: i64,
    pub price: Decimal,
    pub tvl: Decimal,
}

impl SdbImpl for Trade {
    fn table_name() -> &'static str {
        "trades"
    }
    fn field_id(&self) -> i64 {
        self.id
    }
}
impl SdbImpl for TradePrice {
    fn table_name() -> &'static str {
        "trade_prices"
    }
    fn field_id(&self) -> i64{
        self.id
    }
}

#[derive(Clone)]
pub struct TradeRequest {
    pub trade: Trade,
    pub instructions: Vec<Instruction>,

}

#[derive(Debug)]
pub struct TradeResponse {
    pub instructions: Vec<Instruction>,
    pub trade: Trade,
    pub sig: String
}

pub type TradeResponseResult = Result<TradeResponse, TradeResponseError>;

#[repr(i32)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash, Default)]
pub enum TradeState {
    Init = 1,
    PositionRequest = 2,
    PendingBuy = 3,
    PositionFilled = 4,
    PositionPendingClose = 5,
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
    pub async fn send_tx<T:SendTx>(self,rpc:T, send_tx_cfg:Option<SendTxConfig>)->Result<TradeResponse,TradeResponseError>{
        let trade = self.trade.clone();
        let tx = if let Some(send_tx_cfg) = &send_tx_cfg {
            let jito_instructions = vec![
                &self.instructions[..],
                &vec![
                    solana_sdk::system_instruction::transfer(
                        &self.trade.root_kp().pubkey(),
                        &solana_sdk::pubkey::Pubkey::from_str_const(crate::constant::JITO_TIPS[fastrand::usize(..crate::constant::JITO_TIPS.len())]),
                        sol_to_lamports(send_tx_cfg.jito_tip.to_f64().unwrap()),
                    ),
                ][..]
            ].concat();
            Transaction::new_signed_with_payer(
                &jito_instructions,
                Some(&self.trade.root_kp().pubkey()),
                &[self.trade.root_kp()],
                T::get_latest_blockhash().await.expect("yoo"),
            )
        } else {
            Transaction::new_signed_with_payer(
                &self.instructions,
                Some(&self.trade.root_kp().pubkey()),
                &[self.trade.root_kp()],
                T::get_latest_blockhash().await.expect("yoo"),
            )
        };

        // self.trade.state = TradeState::PositionPendingFill;

        rpc.send_tx(tx,send_tx_cfg).await
            .map(|sig|TradeResponse {
                trade: self.trade,
                instructions: self.instructions,
                sig,
            })
            .map_err(|error|TradeResponseError{
                trade,error
            })
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
    pub async fn get_by_pool(pg: &Sdb, data: String) -> Self {
        let sql = format!("SELECT * FROM trades WHERE pool_id = {}",data);
        let a =  pg.query(sql).await.unwrap().take::<Option<Trade>>(0).unwrap();
        a.unwrap()
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
        let coin_mint = self.coin_mint();
        let pc_mint = self.pc_mint();
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
            parent_root_ix.push(ComputeBudgetInstruction::set_compute_unit_limit(90_000));
            let mut create_token_accounts = vec![];
            let mut create_token22_accounts = vec![];
            let mut rest_ix = vec![];

            // let jito_tip_amount = sol_to_lamports(cfg.jito.to_f64().unwrap());
            let priority_fee_amount = cfg.priority_fee;

            // 1. https://solscan.io/tx/5ERUd3HpkQwhBNoTi82YVWFTdAz4Y5Xhjst69LFSiARFdxiGJtZELAPJtrdZQqgkjDnXbb2zVBnGCnqCT9Z5G5yn
            // slippage higher than amount
            // 2. https://solscan.io/tx/4eu6cNN8b8EtDSYcaqco1BbEirVcEDep1TTnd2v2cAJXKsKC6VCWw4mavC3YN2MoFyPvpMMCQL5XcuQ36W1wQafB
            // amount higher than slippage
            // 1. got exact price
            let decimals = self.decimals;

            let sol_trn_normal = cfg.total_normal();
            let sol_trn_minus_fees_normal = cfg.total_minus_fees_normal();
            let amount_normal = (cfg.min_sol / price) * dec!(10).powd(Decimal::from(decimals));
            let slippage_normal = (cfg.max_sol / price) * dec!(10).powd(Decimal::from(decimals));
            // let amount_normal = cfg.min_sol * Decimal::from(LAMPORTS_PER_SOL);
            // let slippage_normal = (cfg.max_sol / price) * dec!(10).powd(Decimal::from(decimals));
            let ca = if token_program_id == spl_token::id() {
                &mut create_token_accounts
            } else {
                &mut create_token22_accounts
            };

            parent_root_ix.extend_from_slice(&vec![
                ComputeBudgetInstruction::set_compute_unit_price(priority_fee_amount.to_u64().unwrap()),
            ]);

            // IT WILL TAKE FEES HERE
            // https://solscan.io/tx/27axTimhcskhx8gnphFtu6LssVkV8qbzgaDNm6v1SNagZhPiY3MzDzDrYqPTdfC5gdUhvJVimBpueQ2ygj9wpaS3
            if ca.is_empty() {
                ca.extend_from_slice(&vec![
                    spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                        &pubkey,
                        &pubkey,
                        &coin_mint,
                        &token_program_id,
                    ),
                    spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                        &pubkey,
                        &pubkey,
                        &pc_mint,
                        &token_program_id,
                    )
                ]);
            }

            rest_ix.extend_from_slice(&vec![
                solana_sdk::system_instruction::transfer(
                    &pubkey,
                    &wsol,
                    (sol_trn_minus_fees_normal * Decimal::from(LAMPORTS_PER_SOL))
                        .to_u64()
                        .unwrap(),
                ),
                spl_token::instruction::sync_native(&token_program_id, &wsol).unwrap(),
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
                    slippage_normal.to_u64().unwrap(),
                    amount_normal.to_u64().unwrap(),
                )
                    .unwrap(),
                // build_memo_instruction(),
                solana_sdk::system_instruction::transfer(
                    &kp.pubkey(),
                    &jito_acc,
                    sol_to_lamports(cfg.jito.to_f64().unwrap()),
                ),
                // solana_sdk::system_instruction::transfer(&ix_pubkey, &jito, jito_tip_amount),
            ]);

            TradeRequest {
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
            }
        } else {
            parent_root_ix.push(ComputeBudgetInstruction::set_compute_unit_limit(60_000));

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
                    &wsol,
                    &kp.pubkey(),
                    &kp.pubkey(),
                    &[&kp.pubkey()],
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
                solana_sdk::system_instruction::transfer(
                    &kp.pubkey(),
                    &jito_acc,
                    sol_to_lamports(cfg.jito.to_f64().unwrap()),
                ),
            ]);
            TradeRequest{
                trade: Trade {
                    ..self
                },
                instructions:parent_root_ix,
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




