use anyhow::anyhow;
use crate::constant::{solana_rpc_client, rpcs, rpc1, rpc2, rpc3, jito1, jito2};
use crate::trade::{TradeRequest, TradeResponse, TradeState};
use base64::Engine;
use futures::future::join_all;
use futures::stream::FuturesUnordered;
use log::info;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde_json::json;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::hash::Hash;
use solana_sdk::native_token::sol_to_lamports;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use tokio::task::JoinHandle;

#[derive(Clone,Copy)]
pub struct SendTxConfig {
    pub jito_tip:Decimal,
}

pub trait SendTx {
    async fn send_tx(&self,tx:Transaction,cfg:Option<SendTxConfig>)->anyhow::Result<String>;
    async fn get_latest_blockhash() -> anyhow::Result<Hash> {
        rpc1().get_latest_blockhash().await.map_err(|err| anyhow::Error::msg(err.to_string()))
    }}

impl SendTx for RpcClient {
    async fn send_tx(&self, tx: Transaction, cfg: Option<SendTxConfig>) -> anyhow::Result<String> {
        let start_time = ::std::time::Instant::now();

        let resp = self
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
            .await.map(|x|x.to_string())
            .map_err(|err| anyhow::Error::msg(err.to_string()));
        info!("{} Trade sent tx: {:?}",self.url(), start_time.elapsed());
        resp
    }


}

impl SendTx for jito_sdk_rust::JitoJsonRpcSDK {
    async fn send_tx(&self,tx:Transaction,cfg:Option<SendTxConfig>) -> anyhow::Result<String> {
        let start_time = ::std::time::Instant::now();
        let serialized_tx =
            base64::engine::general_purpose::STANDARD.encode(bincode::serialize(&tx).unwrap());
        let params = json!({
        "tx": serialized_tx,
        "skipPreflight": false
    });
        let response = self.send_txn(Some(params), true).await.unwrap();
        let sig = response["result"]
            .as_str()
            .ok_or_else(|| anyhow::format_err!("Failed to get signature from response"))
            .unwrap();
        info!("jito Trade sent tx: {:?}", start_time.elapsed());
        Ok(sig.to_string())
    }


}

pub async fn send_tx(mut req: TradeRequest, cfg:Option<SendTxConfig>) -> FuturesUnordered<JoinHandle<anyhow::Result<TradeResponse>>> {
    let mut futs = FuturesUnordered::new();
    futs.push(
        tokio::spawn({
            let req= req.clone();
            async move {
                req.send_tx(rpc1(),cfg).await
            }
        })
    );
    futs.push(
        tokio::spawn({
            let req= req.clone();
            async move {
                req.send_tx(rpc2(),cfg).await
            }
        })
    );
    futs.push(
        tokio::spawn({
            let req= req.clone();
            async move {
                req.send_tx(rpc3(),cfg).await
            }
        })
    );
    futs.push(
        tokio::spawn({
            let req= req.clone();
            async move {
                req.send_tx(jito1(),cfg).await
            }
        })
    );
    futs.push(
        tokio::spawn({
            let req= req.clone();
            async move {
                req.send_tx(jito2(),cfg).await
            }
        })
    );
    futs
}
