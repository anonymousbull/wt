use tokio_rustls::rustls;
use tokio_rustls::rustls::crypto::CryptoProvider;
use wolf_trader::trade_api;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::new().default_filter_or("info"))
        .format_timestamp_millis()
        .init();
    CryptoProvider::install_default(rustls::crypto::ring::default_provider()).unwrap();
    trade_api::start(443).await;
}


