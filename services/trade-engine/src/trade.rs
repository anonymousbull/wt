use crate::constant::{RAYDIUM_V4_AUTHORITY, RAYDIUM_V4_PROGRAM, SOLANA_MINT, SOLANA_MINT_STR};
use crate::rpc::{rpc1, Rpc, RpcResponse, RpcResponseData, RpcResponseMetric, RpcState, RpcType, TradeRpcLog, TradeRpcLogGeneral, TradeRpcLogJito, TradeRpcLogStatus};
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
use std::fmt;
use std::str::FromStr;
use anyhow::{anyhow, Error};
use base64::Engine;
use futures::stream::FuturesUnordered;
use log::{error, info};
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde_json::json;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::commitment_config::CommitmentLevel;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use macros::{implement_mongo_crud, implement_mongo_crud_struct};
use crate::amm_type::TradePool;
use crate::plog::ProgramLog;
use crate::cfg_type::PositionConfig;
use crate::internal::TradeInternal;
use crate::state::TradeState;

#[derive(Clone, Debug,Serialize,Deserialize,JsonSchema)]
pub struct Trade {
    #[schemars(skip)]
    /// id of trade
    pub id: i64,

    #[schemars(skip)]
    pub amount: Decimal,
    #[schemars(skip)]
    /// buy time of trade in ISO 8601 combined date and time with time zone format
    pub buy_time: Option<DateTime<Utc>>,
    /// buy price of trade
    #[schemars(skip)]
    pub buy_price: Option<Decimal>,
    #[schemars(skip)]
    /// sell time of trade in ISO 8601 combined date and time with time zone format
    pub sell_time: Option<DateTime<Utc>>,
    #[schemars(skip)]
    pub sell_price: Option<Decimal>,

    /// pnl percentage of trade
    #[schemars(skip)]
    pub pct: Decimal,
    /// state of trade
    pub state: TradeState,

    #[schemars(skip)]
    pub sol_before: Decimal,
    #[schemars(skip)]
    pub sol_after: Option<Decimal>,
    #[schemars(skip)]
    pub root_kp:Vec<u8>,
    /// user id
    pub user_id:String,
    /// Asset to trade
    pub amm: TradePool,
    #[schemars(skip)]
    pub user_wallet: String,
    #[schemars(skip)]
    pub price:Decimal,
    #[schemars(skip)]
    pub k: Decimal,
    #[schemars(skip)]
    pub tvl:Decimal,

    /// trade configuration settings
    pub cfg: PositionConfig,
    pub error: Option<String>,
    #[schemars(skip)]
    pub plog:ProgramLog,
    #[schemars(skip)]
    pub buy_out: Option<u64>,

    #[schemars(skip)]
    /// internal trade data, null by default
    pub internal: Option<TradeInternal>
}

implement_mongo_crud_struct!(Trade);



#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(tag = "name", content = "arguments")]
pub enum TradeRequest2 {
    /// Initiates a buy trade
    Buy(Trade),

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

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TradeBuySuccessLog {
    pub solscan:String,
    pub url: String,
    pub pct: f32
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
    pub user_wallet: Pubkey,
    pub log: String,
}









