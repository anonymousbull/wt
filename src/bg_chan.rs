use std::collections::HashMap;
use tokio::sync::mpsc::Receiver;
use chrono::Utc;
use log::info;
use reqwest::Client;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::cmd::InternalCommand;
use crate::trade::{Trade, TradePrice};

pub async fn bg_chan(mut rec: Receiver<InternalCommand>) {
    let client = Client::new();
    let mut trades_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("trades.json")
        .await.unwrap();
    let mut contents = String::new();
    trades_file.read_to_string(&mut contents).await.unwrap();
    let trades = serde_json::from_str::<Vec<Trade>>(&contents)
        .unwrap_or(vec![]);
    let mut trades = trades
        .into_iter()
        .map(|x| (x.id.clone(), x))
        .collect::<HashMap<_, _>>();

    while let Some(message) = rec.recv().await {
        match message {
            InternalCommand::LogTrade(mut log) => {
                info!("sending to log server");
                log.dt = Utc::now();
                client.post("https://in.logs.betterstack.com")
                    .header("Authorization", "Bearer dCmCyuBZBcCNPfujgErWZtSf")
                    .header("Content-Type", "application/x-ndjson")
                    .json(&log)
                    .send()
                    .await.unwrap();
                trades.insert(log.trade.id,log.trade.clone());
                let d = trades.values().collect::<Vec<_>>();
                trades_file.write_all(serde_json::to_string(&d).unwrap().as_ref()).await.unwrap();
            }
            InternalCommand::InsertTrade(trade) => {
                // trade.insert(&db).await.unwrap();
            }
            InternalCommand::InsertPrice(price) => {
                // insert_prices.push(price);
                // insert_prices.clear();
                // if insert_prices.len() > 20 {
                //     TradePrice::insert_bulk(&mut c, insert_prices).await.unwrap();
                //     insert_prices = vec![];
                // }
            }
            _ => {}
        }
    }
}