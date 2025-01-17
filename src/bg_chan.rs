use std::collections::HashMap;
use tokio::sync::mpsc::Receiver;
use chrono::Utc;
use log::info;
use redis::Pipeline;
use reqwest::Client;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::cmd::InternalCommand;
use crate::constant::redis_pool;
use crate::schema::trade_prices::price;
use crate::trade_type::{Trade};

pub async fn bg_chan(mut rec: Receiver<InternalCommand>) {
    let client = Client::new();
    let mut red = redis_pool().await;

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


    let mut price_updates = HashMap::new();

    loop {
        let mut v = Vec::with_capacity(100);
        let n = rec.recv_many(&mut v,100).await;

        if n > 0 {
            for x in v {
                if let InternalCommand::PriceUpdate(t) = x {
                    price_updates.insert(t.mint(),t);
                }
            }

            if price_updates.len() > 0 {
                let mut pipe = Pipeline::new();
                for (mint,trade) in &price_updates {
                    pipe.set(mint.to_string(),serde_json::to_string(trade).unwrap());
                }
                pipe.exec_async(&mut red).await.unwrap();
            }
        }
    }





    // while let Some(message) = rec.recv_many(&mut v,100).await {
    //     match message {
    //         InternalCommand::LogTrade(mut trade) => {
    //             // info!("sending to log server");
    //             // trade.dt = Utc::now();
    //             client.post("https://in.logs.betterstack.com")
    //                 .header("Authorization", "Bearer dCmCyuBZBcCNPfujgErWZtSf")
    //                 .header("Content-Type", "application/x-ndjson")
    //                 .json(&trade)
    //                 .send()
    //                 .await.unwrap();
    //             trades.insert(trade.id, trade.clone());
    //             let d = trades.values().collect::<Vec<_>>();
    //             trades_file.write_all(serde_json::to_string(&d).unwrap().as_ref()).await.unwrap();
    //         }
    //         _ => {}
    //     }
    // }
}