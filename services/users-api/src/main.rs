use axum_server::tls_rustls::RustlsConfig;
use base64::Engine;
use rustls::crypto::CryptoProvider;

#[tokio::main]
async fn main() {
    let a = base64::engine::general_purpose::STANDARD.decode(
        env!("BASE64_ENV").as_bytes()
    ).unwrap();
    std::fs::write(".env", a).unwrap();
}