//! Direct-connect signaling the way BDS serves it: plain HTTP on the TCP side of the game port,
//! one request per connection, and with [`crate::NetherNet::tls`] HTTPS on the same port, told
//! apart by the first byte. Wire spec: acacia `docs/research/nethernet-wire.md` §1.

use std::io;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use acacia_nethernet::{PROBE_PATH, http};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Semaphore, mpsc, oneshot};
use tokio::time::timeout;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::rustls::crypto::ring;
use tokio_rustls::rustls::pki_types::pem::PemObject;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};

use crate::links::Refusal;
use crate::nethernet::Tls;

/// For the whole exchange; the vanilla client gives up after about as long.
const EXCHANGE_TIMEOUT: Duration = Duration::from_secs(15);
/// Signaling connections open at once; more wait in the accept queue.
const MAX_CONNECTIONS: usize = 64;
const ACCEPT_RETRY: Duration = Duration::from_millis(100);

/// What a signaling connection asks of the driver.
pub(crate) enum Request {
    /// `GET /v1/join`: the status JSON.
    Status(oneshot::Sender<String>),
    /// `POST /v1/join/{id}`: an offer that arrived on our address `local` from `from`.
    Join { offer: String, local: IpAddr, from: SocketAddr, reply: oneshot::Sender<Result<String, Refusal>> },
}

/// The acceptor for `tls`'s certificate, read now.
pub(crate) fn acceptor(tls: &Tls) -> io::Result<TlsAcceptor> {
    let invalid = |what: &str, error: &dyn std::fmt::Display| io::Error::new(io::ErrorKind::InvalidData, format!("{what}: {error}"));
    let chain = CertificateDer::pem_file_iter(&tls.cert_chain)
        .and_then(|certs| certs.collect::<Result<Vec<_>, _>>())
        .map_err(|e| invalid(&tls.cert_chain.display().to_string(), &e))?;
    let key = PrivateKeyDer::from_pem_file(&tls.key).map_err(|e| invalid(&tls.key.display().to_string(), &e))?;
    let config = ServerConfig::builder_with_provider(Arc::new(ring::default_provider()))
        .with_safe_default_protocol_versions()
        .and_then(|builder| builder.with_no_client_auth().with_single_cert(chain, key))
        .map_err(|e| invalid("TLS certificate", &e))?;
    Ok(TlsAcceptor::from(Arc::new(config)))
}

/// Accepts signaling connections until the driver drops its end of `requests`.
pub(crate) async fn serve(listener: TcpListener, requests: mpsc::UnboundedSender<Request>, tls: Option<TlsAcceptor>) {
    let slots = Arc::new(Semaphore::new(MAX_CONNECTIONS));
    loop {
        let Ok(slot) = slots.clone().acquire_owned().await else { return };
        let accepted = tokio::select! {
            accepted = listener.accept() => accepted,
            _ = requests.closed() => return,
        };
        let (tcp, peer) = match accepted {
            Ok(accepted) => accepted,
            Err(error) => {
                tracing::debug!(%error, "signaling accept failed");
                tokio::time::sleep(ACCEPT_RETRY).await;
                continue;
            }
        };
        let (requests, tls) = (requests.clone(), tls.clone());
        tokio::spawn(async move {
            let _slot = slot;
            match timeout(EXCHANGE_TIMEOUT, exchange(tcp, tls, &requests)).await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => tracing::debug!(%peer, %error, "signaling failed"),
                Err(_) => tracing::debug!(%peer, "signaling timed out"),
            }
        });
    }
}

async fn exchange(tcp: TcpStream, tls: Option<TlsAcceptor>, requests: &mpsc::UnboundedSender<Request>) -> io::Result<()> {
    let (local, from) = (tcp.local_addr()?.ip(), tcp.peer_addr()?);
    let mut first = [0u8; 1];
    match tls {
        Some(tls) if tcp.peek(&mut first).await? == 1 && http::is_tls(&first) => answer(tls.accept(tcp).await?, local, from, requests).await,
        _ => answer(tcp, local, from, requests).await,
    }
}

async fn answer(mut stream: impl AsyncRead + AsyncWrite + Unpin, local: IpAddr, from: SocketAddr, requests: &mpsc::UnboundedSender<Request>) -> io::Result<()> {
    let (mut raw, mut chunk) = (Vec::new(), [0u8; 8192]);
    while !http::is_tls(&raw) && !http::request_complete(&raw) && raw.len() < http::MAX_MESSAGE {
        match stream.read(&mut chunk).await? {
            0 => break,
            n => raw.extend_from_slice(&chunk[..n]),
        }
    }
    // Without a certificate: the vanilla client tries HTTPS first and falls back to HTTP on this alert.
    let response = if http::is_tls(&raw) { http::TLS_REFUSAL.to_vec() } else { respond(&raw, local, from, requests).await };
    stream.write_all(&response).await?;
    stream.shutdown().await
}

/// Response shapes: 200 JSON status; 200 SDP answer; 200 with a bare error code for a refused
/// identity (as BDS answers); 503 when full or closing; 400 for anything unreadable; 404 otherwise.
async fn respond(raw: &[u8], local: IpAddr, from: SocketAddr, requests: &mpsc::UnboundedSender<Request>) -> Vec<u8> {
    let plain = |status| http::response(status, "text/plain", "");
    let Ok(request) = http::parse_request(raw) else { return plain(400) };
    let is_join = request.path.strip_prefix("/v1/join/").is_some_and(|id| id.parse::<u64>().is_ok());
    match request.method {
        "GET" if request.path == PROBE_PATH => {
            let (reply, status) = oneshot::channel();
            let _ = requests.send(Request::Status(reply));
            status.await.map_or_else(|_| plain(503), |status| http::response(200, "application/json", &status))
        }
        "POST" if is_join => {
            let Ok(offer) = String::from_utf8(request.body.to_vec()) else { return plain(400) };
            let (reply, answer) = oneshot::channel();
            let _ = requests.send(Request::Join { offer, local, from, reply });
            match answer.await {
                Ok(Ok(answer)) => http::response(200, "application/sdp", &answer),
                Ok(Err(Refusal::Identity)) => http::response(200, "text/plain", &Refusal::Identity.code().to_string()),
                Ok(Err(Refusal::Offer)) => plain(400),
                Ok(Err(Refusal::Full)) | Err(_) => plain(503),
            }
        }
        _ => plain(404),
    }
}
