use std::sync::Arc;
use env_logger::Builder;
use env_logger::fmt::style;
use log::{Level, LevelFilter, Metadata, Record};
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use wolf_trader::bg_chan::bg_chan;
use wolf_trader::chan::Chan;
use wolf_trader::trade_chan::trade_chan;
use wolf_trader::cmd::InternalCommand;
use wolf_trader::constant::{ENABLE_WEBSOCKET};
use wolf_trader::jito_chan::jito_chan;
use wolf_trader::websocket_server::{start_websocket_server, WebsocketState};

struct SimpleLogger {
    ws:Sender<InternalCommand>
}

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {

            let style = match record.level() {
                Level::Trace => style::AnsiColor::Cyan.on_default(),
                Level::Debug => style::AnsiColor::Blue.on_default(),
                Level::Info => style::AnsiColor::Green.on_default(),
                Level::Warn => style::AnsiColor::Yellow.on_default(),
                Level::Error => style::AnsiColor::Red
                    .on_default()
                    .effects(style::Effects::BOLD),
            };
            let f = format!("[{} {style}{}{style:#}  {}] {}",
                            chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                            record.level(),
                            module_path!(),
                            record.args());
            println!("{f}");
            self.ws.try_send(InternalCommand::Log(f)).unwrap();
        }
    }

    fn flush(&self) {}
}


#[tokio::main]
async fn main() {

    tokio_rustls::rustls::crypto::aws_lc_rs::default_provider().install_default().expect("Failed to install rustls crypto provider");

    // raydium_amm::log::decode_ray_log("Alh4A5VLFQcAWHgDlUsVBwCInOJavHfuY6VQ+YmAAAAAWEKe0EsVBwAANHZmewAAAAAAAAAAAAAAAABuZ0NaFWgAAAAAAAAAAAeCzW9zdO5j/xb1iYAAAAA=");
    raydium_amm::log::decode_ray_log("BHh3Uvk8AAAAYCx1xzAAAAACAAAAAAAAAADh9QUAAAAAI667pxAAAACSCwf8EtAAABT66gMAAAAA");


    let pump = "vdt/007mYe7P926dXkchooyFXXnJov3RsBPN74bgvPMGeX2iI98Gz4CWmAAAAAAAS1ceeQsAAAABqYJJd/7/sFhzhEN5QbMYeJ1RuKF1GQJPkhiNsGdGmFwMjXJnAAAAALokx9ESAAAAGibzfDRqAQC6eKPVCwAAABqO4DCjawAA";
    // let bytes = base64::prelude::BASE64_STANDARD.decode(pump).unwrap();

    raydium_amm::log::decode_ray_log("BHh3Uvk8AAAAYCx1xzAAAAACAAAAAAAAAADh9QUAAAAAI667pxAAAACSCwf8EtAAABT66gMAAAAA");
    solana_sdk::pubkey::Pubkey::from_str_const("So11111111111111111111111111111111111111112");
    let (bg_send, bg_r) = tokio::sync::mpsc::channel::<InternalCommand>(100);
    let (trade_send, trade_r) = tokio::sync::mpsc::channel::<InternalCommand>(500);
    let (ws_s, ws_r) = tokio::sync::mpsc::channel::<InternalCommand>(500);

    if ENABLE_WEBSOCKET == "n" {
        env_logger::Builder::from_env(env_logger::Env::new().default_filter_or("info"))
            .format_timestamp_millis()
            .init();
    } else {
        log::set_boxed_logger(Box::new(SimpleLogger{ws:ws_s.clone()}))
            .map(|()| log::set_max_level(LevelFilter::Info))
            .unwrap();
    }


    let chan = Chan{
        bg: bg_send,
        trade: trade_send,
        ws: ws_s,
    };

    tokio::spawn(async move {
        bg_chan(bg_r).await
    });

    tokio::spawn({
        let chan = chan.clone();
        async move {
            jito_chan(chan).await
        }
    });


    if ENABLE_WEBSOCKET == "n" {
        let c = chan.clone();
        trade_chan(c,trade_r).await;
    } else {
        let c = chan.clone();
        tokio::spawn(async move {
            trade_chan(c,trade_r).await;
        });
        start_websocket_server(Arc::new(WebsocketState{
            chan,
            rec: Mutex::new(ws_r),
        })).await;
    }
}


