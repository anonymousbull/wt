use std::sync::Arc;
use tokio::sync::Mutex;
use wolf_trader::bg_chan::bg_chan;
use wolf_trader::chan::Chan;
use wolf_trader::cmd::InternalCommand;
use wolf_trader::jito_chan::jito_chan;
use wolf_trader::trade_chan::trade_chan;
use wolf_trader::websocket_server::{start_websocket_server, WebsocketState};

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::new().default_filter_or("info"))
        .format_timestamp_millis()
        .init();
    tokio_rustls::rustls::crypto::aws_lc_rs::default_provider().install_default().expect("Failed to install rustls crypto provider");

    // raydium_amm::log::decode_ray_log("Alh4A5VLFQcAWHgDlUsVBwCInOJavHfuY6VQ+YmAAAAAWEKe0EsVBwAANHZmewAAAAAAAAAAAAAAAABuZ0NaFWgAAAAAAAAAAAeCzW9zdO5j/xb1iYAAAAA=");
    raydium_amm::log::decode_ray_log("BHh3Uvk8AAAAYCx1xzAAAAACAAAAAAAAAADh9QUAAAAAI667pxAAAACSCwf8EtAAABT66gMAAAAA");


    // let pump = "vdt/007mYe7P926dXkchooyFXXnJov3RsBPN74bgvPMGeX2iI98Gz4CWmAAAAAAAS1ceeQsAAAABqYJJd/7/sFhzhEN5QbMYeJ1RuKF1GQJPkhiNsGdGmFwMjXJnAAAAALokx9ESAAAAGibzfDRqAQC6eKPVCwAAABqO4DCjawAA";

//     let pump = "vdt/007mYe5zR/rKvm11proPeGwoDBPsrziiklJqgWw+XokSQsxxbwV/0QAAAAAAZbO0JhUAAAAB2ds811Rj9oovAckMF5TfATd+Vf6AVd1SgpLrrhK7kS4ReHhnAAAAACq1zT0QAAAALMcuybKjAQAqCapBCQAAACwvHH0hpQAA";
//     let bytes = base64::prelude::BASE64_STANDARD.decode(pump).unwrap();
//     let log: PumpTradeLog = bincode::deserialize(&bytes[8..]).unwrap();
//     println!("{:?}",log);
//     println!("{:?} {:?}",&bytes[..8], PUMP_BUY_CODE.as_slice());
//
// panic!("d");
    raydium_amm::log::decode_ray_log("BHh3Uvk8AAAAYCx1xzAAAAACAAAAAAAAAADh9QUAAAAAI667pxAAAACSCwf8EtAAABT66gMAAAAA");
    solana_sdk::pubkey::Pubkey::from_str_const("So11111111111111111111111111111111111111112");
    let (bg_send, bg_r) = tokio::sync::mpsc::channel::<InternalCommand>(100);
    let (trade_send, trade_r) = tokio::sync::mpsc::channel::<InternalCommand>(500);
    let (ws_s, ws_r) = tokio::sync::mpsc::channel::<InternalCommand>(500);

    let chan = Chan{
        bg: bg_send,
        trade: trade_send,
        ws: ws_s,
    };

    tokio::spawn(async move {
        bg_chan(bg_r).await
    });

    // tokio::spawn({
    //     let chan = chan.clone();
    //     async move {
    //         jito_chan(chan).await
    //     }
    // });

    let c = chan.clone();
    tokio::spawn(async move {
        trade_chan(c,trade_r).await;
    });

    start_websocket_server(Arc::new(WebsocketState{
        chan,
        rec: Mutex::new(ws_r),
    })).await;
}


