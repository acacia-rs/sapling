//! Direct-connect signaling the way BDS serves it: plain HTTP on the TCP side of the game port,
//! one request per connection. Wire spec: acacia `docs/research/nethernet-wire.md` §1.

use std::io;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use acacia_nethernet::{PROBE_PATH, http};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Semaphore, mpsc, oneshot};
use tokio::time::timeout;

use crate::links::Refusal;

/// For the whole exchange; the vanilla client gives up after about as long.
const EXCHANGE_TIMEOUT: Duration = Duration::from_secs(15);
/// Signaling connections open at once; more wait in the accept queue.
const MAX_CONNECTIONS: usize = 64;
const ACCEPT_RETRY: Duration = Duration::from_millis(100);

/// What a signaling connection asks of the driver.
pub(crate) enum Request {
    /// `GET /v1/join`: the status JSON.
    Status(oneshot::Sender<String>),
    /// `POST /v1/join/{id}`: an offer that arrived on our address `local`.
    Join { offer: String, local: IpAddr, reply: oneshot::Sender<Result<String, Refusal>> },
}

/// Accepts signaling connections until the driver drops its end of `requests`.
pub(crate) async fn serve(listener: TcpListener, requests: mpsc::UnboundedSender<Request>) {
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
        let requests = requests.clone();
        tokio::spawn(async move {
            let _slot = slot;
            match timeout(EXCHANGE_TIMEOUT, exchange(tcp, &requests)).await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => tracing::debug!(%peer, %error, "signaling failed"),
                Err(_) => tracing::debug!(%peer, "signaling timed out"),
            }
        });
    }
}

async fn exchange(mut tcp: TcpStream, requests: &mpsc::UnboundedSender<Request>) -> io::Result<()> {
    let (mut raw, mut chunk) = (Vec::new(), [0u8; 8192]);
    while !http::is_tls(&raw) && !http::request_complete(&raw) && raw.len() < http::MAX_MESSAGE {
        match tcp.read(&mut chunk).await? {
            0 => break,
            n => raw.extend_from_slice(&chunk[..n]),
        }
    }
    // The vanilla client tries HTTPS first and falls back to HTTP on this alert.
    let response = if http::is_tls(&raw) { http::TLS_REFUSAL.to_vec() } else { respond(&raw, tcp.local_addr()?.ip(), requests).await };
    tcp.write_all(&response).await?;
    tcp.shutdown().await
}

/// Response shapes: 200 JSON status; 200 SDP answer; 200 with a bare error code for a refused
/// identity (as BDS answers); 503 when full or closing; 400 for anything unreadable; 404 otherwise.
async fn respond(raw: &[u8], local: IpAddr, requests: &mpsc::UnboundedSender<Request>) -> Vec<u8> {
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
            let _ = requests.send(Request::Join { offer, local, reply });
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
