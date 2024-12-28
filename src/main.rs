use wolf_trader::chan::{bg_chan, trade_chan, Chan};
use wolf_trader::cmd::InternalCommand;


#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::new().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    // raydium_amm::log::decode_ray_log("AwASegAAAAAAAIhSanQAAAABAAAAAAAAAICWmAAAAAAAaDy0k00AAADAKw8ZIwAAAKQfDQEAAAAA");
    // raydium_amm::log::decode_ray_log("BO2eLLAAAAAAJH/wjAAAAAABAAAAAAAAAICWmAAAAAAA+AYIVWGkYgKgtLhGxAAAAGwtAAAAAAAA");
    solana_sdk::pubkey::Pubkey::from_str_const("So11111111111111111111111111111111111111112");
    let (bg_send, bg_r) = tokio::sync::mpsc::channel::<InternalCommand>(100);
    let (trade_send, trade_r) = tokio::sync::mpsc::channel::<InternalCommand>(500);

    let chan = Chan{
        bg: bg_send,
        trade: trade_send,
    };


    tokio::spawn(async move {
        bg_chan(bg_r).await
    });

    trade_chan(chan,trade_r).await;

}


