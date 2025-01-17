use std::time::Duration;
use anyhow::anyhow;
use futures::TryStreamExt;
use mongodb::bson;
use mongodb::bson::{doc, Binary};
use mongodb::bson::spec::BinarySubtype;
use postgrest::Postgrest;
use redis::{Commands, Connection};
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};
use solana_sdk::signature::Keypair;
use tokio::sync::oneshot;
use yellowstone_grpc_proto::tonic::codegen::tokio_stream::StreamExt;
use crate::chan::Chan;
use crate::cmd::InternalCommand;
use crate::constant::{get_keypair, SOLANA_MINT, SUPABASE_URL};
use crate::trade_type::{Trade, TradeInternal, TradePool};


impl Trade {

}