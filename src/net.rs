use fastwebsockets::FragmentCollector;
use http_body_util::Empty;
use hyper::header::{CONNECTION, UPGRADE};
use hyper::upgrade::Upgraded;
use hyper::Request;
use hyper_util::rt::TokioIo;
use std::future::Future;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::rustls::ClientConfig;
use tokio_rustls::TlsConnector;

pub fn tls_connector() -> anyhow::Result<TlsConnector> {
    let root_store = tokio_rustls::rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };
    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    Ok(TlsConnector::from(Arc::new(config)))
}

struct SpawnExecutor;

impl<Fut> hyper::rt::Executor<Fut> for SpawnExecutor
where
    Fut: Future + Send + 'static,
    Fut::Output: Send + 'static,
{
    fn execute(&self, fut: Fut) {
        tokio::task::spawn(fut);
    }
}

pub async fn connect_tls(full_url: &str) -> anyhow::Result<FragmentCollector<TokioIo<Upgraded>>> {
    let url = url::Url::parse(full_url)?;
    let host = url.host().ok_or_else(|| anyhow::anyhow!("Invalid URL"))?;
    let domain = host.to_string();
    let port = url.port().unwrap_or(443);
    let addr = format!("{}:{}", domain, port);

    let tcp_stream = TcpStream::connect(&addr).await?;
    let tls_connector = tls_connector().unwrap();
    let server_name = tokio_rustls::rustls::pki_types::ServerName::try_from(domain.clone())
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid dnsname"))?;

    let tls_stream = tls_connector.connect(server_name, tcp_stream).await?;

    let req = Request::builder()
        .method("GET")
        .uri(url.as_str())
        .header("Host", domain)
        .header(UPGRADE, "websocket")
        .header(CONNECTION, "upgrade")
        .header(
            "Sec-WebSocket-Key",
            fastwebsockets::handshake::generate_key(),
        )
        .header("Sec-WebSocket-Version", "13")
        .body(Empty::<hyper::body::Bytes>::new())?;

    let (ws, _) =
        fastwebsockets::handshake::client(&SpawnExecutor, req, tls_stream).await?;
    Ok(FragmentCollector::new(ws))
}