use crate::cmd::InternalCommand;
use crate::constant::{get_keypair, pg_conn, solana_rpc_client, BLOX_HEADER, PUMP_MIGRATION, RAYDIUM_V4_AUTHORITY, RAYDIUM_V4_PROGRAM, SOLANA_GRPC_URL, SOLANA_MINT_STR, SOLANA_RPC_URL};
use crate::price::{Price, PriceBuilder};
use crate::util::Chan;
use anyhow::anyhow;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use futures::future::{select_all, SelectAll};
use futures::{pin_mut, FutureExt, SinkExt, StreamExt};
use jito_sdk_rust::JitoJsonRpcSDK;
use log::{error, info, warn};
use raydium_amm::instruction::swap_base_out;
use raydium_amm::instruction::AmmInstruction::Initialize2;
use raydium_amm::log::{decode_ray_log, InitLog, LogType, SwapBaseInLog, SwapBaseOutLog};
use raydium_amm::math::SwapDirection;
use reqwest::Client;
use rust_decimal_macros::dec;
use serde_json::{json, Value};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcSendTransactionConfig, RpcTransactionConfig};
use solana_client::tpu_client::TpuClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::message::VersionedMessage;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use solana_transaction_status::UiTransactionEncoding;
use std::cmp::{max, min};
use std::collections::{HashMap, HashSet};
use std::pin::pin;
use std::str::FromStr;
use chrono::{DateTime, Utc};
use diesel::{deserialize, serialize, AsChangeset, AsExpression, FromSqlRow, Identifiable, Insertable, Queryable, Selectable};
use diesel::backend::Backend;
use diesel::deserialize::FromSql;
use diesel::serialize::{Output, ToSql};
use diesel::sql_types::Integer;
use diesel::dsl::count_star;
use diesel::prelude::*;
use diesel_async::pooled_connection::deadpool::Object;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use rust_decimal::{Decimal, MathematicalOps};
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::signature::{Keypair, Signature};
use spl_associated_token_account_client::address::get_associated_token_address_with_program_id;
use yellowstone_grpc_proto::geyser::subscribe_update::UpdateOneof;
use yellowstone_grpc_proto::geyser::{
    SubscribeRequest, SubscribeRequestFilterAccounts, SubscribeRequestFilterBlocksMeta,
    SubscribeRequestFilterTransactions,
};
use yellowstone_grpc_proto::prelude::subscribe_request_filter_accounts_filter::Filter;
use yellowstone_grpc_proto::prelude::SubscribeRequestFilterAccountsFilter;
use yellowstone_grpc_proto::prost::Message;
use raydium_amm::solana_program::native_token::{sol_to_lamports, LAMPORTS_PER_SOL};
use crate::implement_diesel;
use crate::position::PositionConfig;

#[derive(Queryable, Selectable, Insertable, AsChangeset, Identifiable)]
#[diesel(table_name = crate::schema::trades)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Trade {
    pub id: String,
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

    pub tx_in_id:Option<String>,
    pub tx_out_id:Option<String>,
    pub sol_before: Decimal,
    pub sol_after: Option<Decimal>,

    pub root_kp:Vec<u8>,
}

implement_diesel!(Trade, trades);

pub struct TradeRequest {
    pub trade: Trade,
    pub instructions: Vec<Instruction>
}

pub struct TradeResponse {
    pub instructions: Vec<Instruction>,
    pub trade: Trade,
    pub sig: String
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, AsExpression, FromSqlRow, Eq, PartialEq, Hash)]
#[diesel(sql_type = Integer)]
pub enum TradeState {
    Init = 1,
    PositionRequest = 2,
    PositionPendingFill = 3,
    PositionFilled = 4,
    PositionPendingClose = 5,
    PositionClosed = 6,
}


impl<DB> ToSql<Integer, DB> for TradeState
where
    DB: Backend,
    i32: ToSql<Integer, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        match self {
            TradeState::Init => 1.to_sql(out),
            TradeState::PositionRequest => 2.to_sql(out),
            TradeState::PositionPendingFill => 3.to_sql(out),
            TradeState::PositionFilled => 4.to_sql(out),
            TradeState::PositionPendingClose => 5.to_sql(out),
            TradeState::PositionClosed => 6.to_sql(out),
        }
    }
}

impl<DB> FromSql<Integer, DB> for TradeState
where
    DB: Backend,
    i32: FromSql<Integer, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        match i32::from_sql(bytes)? {
            1 => Ok(TradeState::Init),
            2 => Ok(TradeState::PositionRequest),
            3 => Ok(TradeState::PositionPendingFill),
            4 => Ok(TradeState::PositionFilled),
            5 => Ok(TradeState::PositionPendingClose),
            6 => Ok(TradeState::PositionClosed),
            x => Err(format!("Unrecognized variant {}", x).into()),
        }
    }
}





impl Trade {
    pub async fn upsert_bulk(mut pg: &mut Object<AsyncPgConnection>, data: Vec<Self>) -> anyhow::Result<()> {
        let start_time = ::std::time::Instant::now();
        diesel::insert_into(crate::schema::trades::table)
            .values(&data)
            .on_conflict(crate::schema::trades::id)
            .do_update()
            .set((
                crate::schema::trades::pct.eq(diesel::upsert::excluded(crate::schema::trades::pct)),
                crate::schema::trades::entry_price.eq(diesel::upsert::excluded(crate::schema::trades::entry_price)),
                crate::schema::trades::exit_price.eq(diesel::upsert::excluded(crate::schema::trades::exit_price)),
                crate::schema::trades::exit_time.eq(diesel::upsert::excluded(crate::schema::trades::exit_time)),
                crate::schema::trades::state.eq(diesel::upsert::excluded(crate::schema::trades::state)),
                crate::schema::trades::tx_in_id.eq(diesel::upsert::excluded(crate::schema::trades::tx_in_id)),
                crate::schema::trades::tx_out_id.eq(diesel::upsert::excluded(crate::schema::trades::tx_out_id)),
            ))
            .execute(&mut pg).await?;
        let elapsed_time = start_time.elapsed();
        info!("Time taken for bulk upsert: {} {} {:?}",data.len(), "trades", elapsed_time);
        Ok(())
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
        Pubkey::from_str(&self.id.as_str()).unwrap()
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

        let token_program_id = self.token_program();
        let kp = self.root_kp();
        let pubkey = kp.pubkey();
        let coin_mint = self.coin_mint();
        let pc_mint = self.pc_mint();
        let amm = self.amm();
        let coin_tk = get_associated_token_address_with_program_id(
            &pubkey,
            &coin_mint,
            &token_program_id,
        );
        let pc_tk = get_associated_token_address_with_program_id(
            &pubkey,
            &pc_mint,
            &token_program_id,
        );
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
            ComputeBudgetInstruction::set_compute_unit_limit(100_000),
        ];

        if open {
            let mut create_token_accounts = vec![];
            let mut create_token22_accounts = vec![];
            let mut rest_ix = vec![];

            // let jito = Pubkey::from_str_const(JITO_TIPS[fastrand::usize(..JITO_TIPS.len())]);
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
                build_memo_instruction(),
                solana_sdk::system_instruction::transfer(
                    &kp.pubkey(),
                    &Pubkey::from_str_const("HWEoBxYs7ssKuudEjzjmpfJVX7Dvi7wescFsVx2L5yoY"),
                    sol_to_lamports(cfg.jito.to_f64().unwrap()),
                ),
                // solana_sdk::system_instruction::transfer(&ix_pubkey, &jito, jito_tip_amount),
            ]);

            TradeRequest {
                trade: Trade {
                    amount: amount_normal,
                    entry_time: None,
                    entry_price: None,
                    exit_price: None,
                    exit_time: None,
                    pct: Default::default(),
                    state: TradeState::PositionRequest,
                    tx_in_id: None,
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
                spl_token::instruction::close_account(
                    &token_program_id,
                    &wsol,
                    &kp.pubkey(),
                    &kp.pubkey(),
                    &[&kp.pubkey()],
                )
                    .unwrap(),
                build_memo_instruction(),
                solana_sdk::system_instruction::transfer(
                    &kp.pubkey(),
                    &Pubkey::from_str_const("HWEoBxYs7ssKuudEjzjmpfJVX7Dvi7wescFsVx2L5yoY"),
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
    // pub async fn try_fetch_pool(accounts: &[Pubkey]) -> Option<Self> {
    //     let start_time = ::std::time::Instant::now();
    //
    //     let amm = accounts[2].to_string();
    //     let coin_vault = accounts[5].to_string();
    //     let pool_coin_vault = accounts[6].to_string();
    //     let token_program = accounts.iter().find(|&x|{
    //         &spl_token::id() == x || &spl_token_2022::id() == x
    //     });
    //
    //     let rpc = solana_rpc_client();
    //     let response = rpc.get_account(&Pubkey::from_str_const(amm.as_str()))
    //         .await
    //         .unwrap();
    //     let pool = AmmInfo::load_from_bytes(response.data.as_slice()).unwrap();
    //
    //     let pool = Self {
    //         id: amm,
    //         pc_vault: pool.pc_vault.to_string(),
    //         coin_vault: pool.coin_vault.to_string(),
    //         coin_mint:pool.coin_vault_mint.to_string(),
    //         // DANGEROUS
    //         pc_mint: pool.pc_vault_mint.to_string(),
    //         decimals: 6,
    //         token_program_id: token_program.unwrap().to_string(),
    //     };
    //
    //     if pool.is_sol_pool() {
    //         Some(pool)
    //     } else {
    //         None
    //     }
    // }

    pub fn from_solana_account_grpc(accounts: &[Pubkey]) -> Self {
        let token_program_id = accounts[0].to_string();
        let amm = accounts[4].to_string();
        let coin_mint = accounts[8].to_string();
        let pc_mint = accounts[9].to_string();
        let coin_vault = accounts[10].to_string();
        let pc_vault = accounts[11].to_string();
        Self {
            id: amm,
            coin_vault,
            pc_vault,
            coin_mint,
            pc_mint,
            // VERY RISKY
            decimals: 6,
            token_program_id,

            amount: Default::default(),
            entry_time: None,
            entry_price: None,
            exit_price: None,
            exit_time: None,
            pct: Default::default(),
            state: TradeState::Init,
            tx_in_id: None,
            tx_out_id: None,
            sol_before: Default::default(),
            sol_after: None,
            root_kp: vec![],
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




