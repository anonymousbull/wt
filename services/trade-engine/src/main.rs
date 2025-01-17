use trade_engine::bg::bg_chan;
use trade_engine::chan::Chan;
use trade_engine::engine::trade_chan;
use trade_engine::event::InternalCommand;
use trade_engine::server;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::new().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    let (bg_send, bg_r) = tokio::sync::mpsc::channel::<InternalCommand>(100);
    let (trade_send, trade_r) = tokio::sync::mpsc::channel::<InternalCommand>(500);
    let (ws_s, ws_r) = tokio::sync::mpsc::channel::<InternalCommand>(500);
    let (dsl_s, dsl_r) = tokio::sync::mpsc::channel::<InternalCommand>(500);
    let (web_s, web_r) = tokio::sync::mpsc::channel::<InternalCommand>(500);
    let chan = Chan{
        bg: bg_send,
        trade: trade_send,
        dsl: dsl_s,
    };

    tokio::spawn(async move {
        bg_chan(bg_r).await
    });

    tokio::spawn({
        let chan = chan.clone();
        async move {
            trade_chan(chan,trade_r).await;
        }
    });

    server::start(chan).await;
}


