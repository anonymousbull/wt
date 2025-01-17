use crate::constant::{mongo, redis_pool};
use crate::event::InternalCommand;
use redis::Pipeline;
use std::collections::HashMap;
use tokio::sync::mpsc::Receiver;
use crate::trade::Trade;

pub async fn bg_chan(mut rec: Receiver<InternalCommand>) {
    let mut red = redis_pool().await;
    let mut mon = mongo().await;
    let col = mon.collection::<Trade>("trades");
    let mut price_updates = HashMap::new();
    let mut trade_confirmations = HashMap::new();

    loop {
        let mut v = Vec::with_capacity(100);
        let n = rec.recv_many(&mut v,100).await;

        if n > 0 {
            for x in v {
                if let InternalCommand::PriceUpdate(t) = x {
                    price_updates.insert(t.mint(),t);
                } else if let InternalCommand::TradeConfirmation(t) = x {
                    trade_confirmations.insert(t.mint(),t);
                }
            }

            let mut pipe = Pipeline::new();
            if price_updates.len() > 0 {
                for (mint,trade) in &price_updates {
                    pipe.set(mint.to_string(),serde_json::to_string(trade).unwrap());
                }
                price_updates.clear();
            }

            if trade_confirmations.len() > 0 {
                for (_,trade) in &trade_confirmations {
                    trade.db_upsert_mongo(&col).await;
                }
                trade_confirmations.clear();
            }
            pipe.exec_async(&mut red).await.unwrap();
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