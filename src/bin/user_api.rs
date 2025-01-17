use tokio_rustls::rustls;
use tokio_rustls::rustls::crypto::CryptoProvider;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::new().default_filter_or("info"))
        .format_timestamp_millis()
        .init();
    CryptoProvider::install_default(rustls::crypto::ring::default_provider()).unwrap();
    wolf_trader::user_api::start(env!("PORT").parse::<u16>().unwrap()).await;
}