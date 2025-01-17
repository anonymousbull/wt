use axum_server::tls_rustls::RustlsConfig;
use rustls::crypto::CryptoProvider;

#[tokio::main]
async fn main() {
    // tracing::init_default_subscriber();
    CryptoProvider::install_default(rustls::crypto::ring::default_provider()).unwrap();
    let cert = tokio::fs::read(env!("SSL_CERT")).await.unwrap();
    let key = tokio::fs::read(env!("SSL_KEY")).await.unwrap();
    let port = env!("PORT").parse::<u16>().unwrap();
    let ssl = RustlsConfig::from_pem(cert,key).await.unwrap();
    users_api::server::start(ssl,port).await;
}