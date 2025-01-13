use crate::net::connect_tls;
use chrono::{DateTime, Utc};
use fastwebsockets::OpCode;
use log::{error, info};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use crate::chan::Chan;
use crate::cmd::InternalCommand;

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


pub async fn jito_chan(chan:Chan){
    let domain = "https://bundles.jito.wtf/api/v1/bundles/tip_stream";
    let mut ws = connect_tls(domain).await.unwrap();
    info!("connected to websocket server");
    while let Ok(frame) = ws.read_frame().await {

        match frame.opcode {
            OpCode::Text => {
                let tips  = serde_json::from_slice::<Vec<TipStatistics>>(frame.payload.as_ref()).unwrap();
                if let Some(tip) = tips.get(0) {
                    // chan.trade.try_send(InternalCommand::JitoTip(tip.clone())).unwrap();
                }
            }
            _ => {
                error!("received non-text opcode {:?}", frame.opcode);
            }
        }
    }
}