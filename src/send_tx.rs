use base64::Engine;
use jito_sdk_rust::JitoJsonRpcSDK;
use log::info;
use reqwest::Client;
use serde_json::{json, Value};
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use crate::constant::{solana_rpc_client, BLOX_HEADER};
use crate::trade::{TradeRequest, TradeResponse, TradeState};

pub async fn send_tx(mut req: TradeRequest) -> anyhow::Result<TradeResponse> {
    let rpc = solana_rpc_client();
    let start_time = ::std::time::Instant::now();
    let jito_sdk = JitoJsonRpcSDK::new("https://mainnet.block-engine.jito.wtf/api/v1", None);

    let tx = Transaction::new_signed_with_payer(
        &req.instructions,
        Some(&req.trade.root_kp().pubkey()),
        &[req.trade.root_kp()],
        rpc.get_latest_blockhash().await?,
    );
    // let serialized_tx =
    //     base64::engine::general_purpose::STANDARD.encode(bincode::serialize(&tx)?);

    // info!("Sending transaction...");

    // let params = json!({
    //     "tx": serialized_tx,
    //     "skipPreflight": true
    // });

    // let response = jito_sdk.send_txn(Some(params), true).await.unwrap();
    // let sig = response["result"]
    //     .as_str()
    //     .ok_or_else(|| anyhow!("Failed to get signature from response"))
    //     .unwrap();

    let serialized_tx =
        base64::engine::general_purpose::STANDARD.encode(bincode::serialize(&tx).unwrap());
    let client = Client::new();

    let response = client
        .post("https://ny.solana.dex.blxrbdn.com/api/v2/submit")
        .header("Authorization", BLOX_HEADER)
        .json(&json!({
            "transaction": {"content": serialized_tx},
            "frontRunningProtection": false,
            "useStakedRPCs": true,
        }))
        .send()
        .await
        .unwrap();

    let text = response.text().await.unwrap();
    let v = serde_json::from_str::<Value>(&text).unwrap();
    // println!("Status: {}", response.status());
    info!("Body: {}", text);

    let sig = v["signature"].as_str().unwrap();

    req.trade.state = TradeState::PositionPendingFill;
    // info!("Transaction sent with signature: {}", sig);
    // info!("Built trade tx: {:?}", start_time.elapsed());
    //
    // // let start_time = ::std::time::Instant::now();
    // let sig = rpc
    //     .send_transaction_with_config(
    //         &tx,
    //         RpcSendTransactionConfig{
    //             skip_preflight: true,
    //             preflight_commitment: None,
    //             encoding: None,
    //             max_retries: None,
    //             min_context_slot: None,
    //         }
    //         // &rpc_client.get_latest_blockhash().await.unwrap(),
    //     )
    //     .await?;
    // info!("signature {siga} {:?}", start_time.elapsed());

    let elapsed_time = start_time.elapsed();
    info!("Confirmed trade sent tx: {:?}", start_time.elapsed());
    //

    // let sig = rpc
    //     .send_and_confirm_transaction(&tx)
    //     .await
    //     .unwrap();
    info!("this tx is confirmed v2 {:?}", sig);

    Ok(TradeResponse {
        trade: req.trade,
        instructions: req.instructions,
        sig: sig.to_string(),
    })
}
