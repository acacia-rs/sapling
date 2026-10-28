//! HTTPS signaling beside plain HTTP on one port, with a test certificate for localhost
//! (tests/data, self-signed, valid to 2126).

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use p384::ecdsa::SigningKey;
use sapling_net::{NetConfig, NetherNet, SigningKeys, Tls, listen};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::rustls::crypto::ring;
use tokio_rustls::rustls::pki_types::pem::PemObject;
use tokio_rustls::rustls::pki_types::{CertificateDer, ServerName};
use tokio_rustls::rustls::{ClientConfig, RootCertStore};

fn data(file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data").join(file)
}

async fn status(mut stream: impl AsyncRead + AsyncWrite + Unpin, addr: SocketAddr) -> String {
    stream.write_all(format!("GET /v1/join HTTP/1.1\r\nHost: {addr}\r\n\r\n").as_bytes()).await.unwrap();
    let mut response = Vec::new();
    // rustls reports the peer's missing close_notify as an error after the body arrived.
    let _ = stream.read_to_end(&mut response).await;
    String::from_utf8(response).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn https_and_http_share_the_signaling_port() {
    let tls = Tls { cert_chain: data("localhost.crt"), key: data("localhost.key") };
    let nethernet = NetherNet { tls: Some(tls), ..NetherNet::new(SigningKey::from_slice(&[9; 48]).unwrap()) };
    let cfg = NetConfig { online_mode: false, nethernet: Some(nethernet), ..NetConfig::default() };
    let (listener, _outbox, _inbox) = listen("127.0.0.1:0".parse().unwrap(), cfg, SigningKeys::empty()).await.unwrap();
    let addr = listener.local_addr();

    let mut roots = RootCertStore::empty();
    roots.add(CertificateDer::from_pem_file(data("localhost.crt")).unwrap()).unwrap();
    let client = ClientConfig::builder_with_provider(Arc::new(ring::default_provider())).with_safe_default_protocol_versions().unwrap();
    let connector = TlsConnector::from(Arc::new(client.with_root_certificates(roots).with_no_client_auth()));
    let tls = connector.connect(ServerName::try_from("localhost").unwrap(), TcpStream::connect(addr).await.unwrap()).await.unwrap();
    let over_tls = status(tls, addr).await;
    assert!(over_tls.starts_with("HTTP/1.1 200") && over_tls.contains("\"maxPlayers\""), "{over_tls}");

    let plain = status(TcpStream::connect(addr).await.unwrap(), addr).await;
    assert_eq!(plain.split("\r\n\r\n").nth(1), over_tls.split("\r\n\r\n").nth(1));
}
