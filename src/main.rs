use std::sync::Arc;
use tokio::sync::Mutex;
use wolf_trader::chan_bg::bg_chan;
use wolf_trader::prompt_chan::{dsl_chan, export_prompt_schema};
use wolf_trader::chan::Chan;
use wolf_trader::cmd::{BroadcastCommand, InternalCommand};
use wolf_trader::jito_chan::jito_chan;
use wolf_trader::trade_chan::trade_chan;
use wolf_trader::web_server;
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
    let (dsl_s, dsl_r) = tokio::sync::mpsc::channel::<InternalCommand>(500);
    let (web_s, web_r) = tokio::sync::mpsc::channel::<InternalCommand>(500);
    let (ws,_) = tokio::sync::broadcast::channel::<BroadcastCommand>(500);
    export_prompt_schema();
    let chan = Chan{
        bg: bg_send,
        trade: trade_send,
        ws:ws.clone(),
        dsl: dsl_s,
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

    tokio::spawn({
        let chan = chan.clone();
        async move {
            dsl_chan(chan,dsl_r).await;
        }
    });

    tokio::spawn({
        let chan = chan.clone();
        async move {
            web_server::start(chan).await;
        }
    });

    tokio::spawn({
        let chan = chan.clone();
        async move {
            trade_chan(chan,trade_r).await;
        }
    });


    start_websocket_server(Arc::new(WebsocketState{
        chan,
        rec: ws.clone(),
    })).await;
}


