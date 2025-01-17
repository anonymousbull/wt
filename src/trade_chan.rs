use crate::chan::Chan;
use crate::cmd::InternalCommand;
use crate::constant::{get_keypair, get_trade, kakfa_producer, mongo, redis_pool, PUMP_EVENT_AUTHORITY, PUMP_FEE, PUMP_MIGRATION, PUMP_MIGRATION_PRICE, PUMP_PROGRAM, RAYDIUM_V4_AUTHORITY, RAYDIUM_V4_PROGRAM, SOLANA_ATA_PROGRAM, SOLANA_MINT, SOLANA_RENT_PROGRAM, SOLANA_SERUM_PROGRAM, SOLANA_SYSTEM_PROGRAM, SUPABASE_PK, SUPABASE_SK, SUPABASE_URL, USER_API_KEY, USER_API_URL};
use crate::db::{get_ignore_mints, update_ignore_mints};
use crate::position::PositionConfig;
use crate::program_log::{ProgramLog, ProgramLogInfo, PumpTradeLog};
use crate::rpc::{
    geyser, rpcs, solana_rpc_client, RpcResponseData, RpcState, RpcsConfig, TradeRpcLogStatus,
};
use crate::solana::*;
use crate::trade_type::{
    Trade, TradePrice, TradeResponse, TradeResponseError, TradeSignature, TradeState,
};
use crate::app_user_type::UserWithId;
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use log::{error, info, warn};
use postgrest::Postgrest;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use serde_json::json;
use solana_sdk::hash::Hash;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signature};
use std::cmp::PartialEq;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;
use tokio::fs::OpenOptions;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc::Receiver;
use tokio::sync::oneshot::Sender;
use tokio::time::Instant;
use yellowstone_grpc_proto::geyser::subscribe_update::UpdateOneof;
use yellowstone_grpc_proto::geyser::{
    CommitmentLevel, SubscribeRequest, SubscribeRequestFilterTransactions,
    SubscribeUpdateTransactionInfo,
};
use yellowstone_grpc_proto::prelude::{
    Message, SubscribeUpdate, SubscribeUpdateTransaction, Transaction, TransactionStatusMeta,
};
use yellowstone_grpc_proto::tonic::Status;



#[derive(Serialize, Deserialize, Debug, Default)]
pub enum TradeLevel {
    Error = 1,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}




