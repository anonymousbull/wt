use wolf_trader::bg_chan::bg_chan;
use wolf_trader::chan::Chan;
use wolf_trader::trade_chan::trade_chan;
use wolf_trader::trade_cmd::InternalCommand;

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


