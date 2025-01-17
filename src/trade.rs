use crate::constant::{RAYDIUM_V4_AUTHORITY, RAYDIUM_V4_PROGRAM, SOLANA_MINT, SOLANA_MINT_STR};
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
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::commitment_config::CommitmentLevel;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use crate::amm_type::TradePool;
use crate::plog::ProgramLog;
use crate::implement_mongo_crud_struct;
use crate::swap_config::PositionConfig;
use crate::trade_rpc::*;


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

    pub instructions: Vec<TradeInstruction>,
    pub transactions: Vec<TradeTransaction>,
    pub rpc_status: RpcState,
    pub rpc_logs:Vec<TradeRpcLog>,
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






#[derive(Clone,Serialize,Deserialize,Debug)]
pub enum TradeInstruction {
    Jito(Vec<Instruction>),
    General(Vec<Instruction>)
}

#[derive(Clone,Serialize,Deserialize,Debug, PartialEq)]
pub enum TradeTransaction {
    Jito(Transaction),
    General(Transaction)
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash, Default, JsonSchema)]
pub enum TradeState {
    #[default]
    /// trade is in buy state with intent to buy later
    Buy = 1,
    /// trade has been placed and pending buy confirmation
    PendingBuy,
    /// trade has been placed and buy was successful
    BuySuccess,
    /// trade is in sell state with intent to sell previous buy trade later
    PendingSell,
    /// trade has been placed and sell was successful
    SellSuccess,
    /// trade has been placed and buy was failed
    BuyFailed,
    /// trade has been placed and sell was failed
    SellFailed
}


#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct RpcInfo {
    pub name:String,
    pub location: String
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct RpcResponse {
    pub signature:String,
    pub metric:RpcResponseMetric,
    pub rpc_info:RpcInfo,
    pub rpc_response_data: RpcResponseData
}

#[derive(Clone,Debug,Serialize,Deserialize,PartialEq, Eq)]
pub enum TradeRpcLog {
    BuyJito(TradeRpcLogJito,TradeRpcLogStatus),
    SellJito(TradeRpcLogJito,TradeRpcLogStatus),
    BuyGeneral(TradeRpcLogGeneral,TradeRpcLogStatus),
    SellGeneral(TradeRpcLogGeneral,TradeRpcLogStatus),
}

#[derive(Clone,Debug,Serialize,Deserialize,PartialEq,Eq)]
pub enum TradeRpcLogStatus {
    Pending,
    Success,
    Fail,
}

#[derive(Debug, Serialize, Deserialize,Clone,Copy)]
pub struct TipStatistics {
    pub time: DateTime<Utc>,
    pub landed_tips_25th_percentile: Decimal,
    pub landed_tips_50th_percentile: Decimal,
    pub landed_tips_75th_percentile: Decimal,
    pub landed_tips_95th_percentile: Decimal,
    pub landed_tips_99th_percentile: Decimal,
    pub ema_landed_tips_50th_percentile: Decimal,
}


#[derive(Clone,Debug,Serialize,Deserialize,PartialEq,Eq)]
pub struct TradeRpcLogJito {
    pub general:TradeRpcLogGeneral,
    pub tip_sol_ui: String
}

#[derive(Clone,Debug,Serialize,Deserialize,PartialEq,Eq)]
pub struct TradeRpcLogGeneral {
    pub signature:String,
    pub response_time:String,
    pub name:String,
    pub tx: Transaction,
    pub ix: Vec<Instruction>,
}

#[derive(Clone,Debug,Serialize,Deserialize,Default,PartialEq,Eq,JsonSchema)]
pub enum RpcState {
    #[default]
    Free=1,
    Busy
}

#[derive(Clone,Debug,Serialize,Deserialize,Copy)]
pub enum RpcResponseData {
    Jito {
        tip_amount_sol:Decimal,
        priority_amount_micro_sol:Decimal,
    },
    General {
        pfee_sol_ui:Decimal,
    }
}



#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct RpcResponseMetric {
    pub response_time:u128,
    pub response_time_string:String,
}

pub enum RpcType {
    Jito,
    General
}



pub enum TradeRpc {
    Jito {
        rpc: jito_sdk_rust::JitoJsonRpcSDK,
        tip: Option<TipStatistics>,
        info: RpcInfo
    },
    General {
        rpc: RpcClient,
        info: RpcInfo
    }
}


#[derive(Clone,Debug,Serialize,Deserialize,Default,Copy)]
pub struct RpcsConfig {
    pub jito_tip: Option<TipStatistics>,
}



