use std::io;
use std::net::SocketAddr;
use std::path::Path;
use std::time::Instant;

use acacia_auth::login::verify::{SigningKeys, Verifier};
use acacia_nethernet::host::{Host, HostConfig};
use acacia_raknet::{Server, ServerConfig};
use p384::ecdsa::SigningKey;
use rand_core::RngCore;
use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::config::NetConfig;
use crate::congestion::Congestion;
use crate::driver::Driver;
use crate::event::{Inbox, Outbox};
use crate::lan::Lan;
use crate::links::Links;
use crate::router::Router;
use crate::sessions::Sessions;

/// Connections allowed beyond `max_players`, so a full server can still tell newcomers it is full.
const HANDSHAKE_SLACK: usize = 32;
/// Tries at finding a free port number that is free on both UDP and TCP, when asked for port 0.
const PAIR_ATTEMPTS: usize = 16;

/// The listening sockets and the task driving them. The task ends when the game drops the
/// [`Outbox`] or the [`Inbox`]; players are then told the server closed.
pub struct Listener {
    addr: SocketAddr,
    lan: Option<Advert>,
    task: JoinHandle<()>,
}

/// Where and as whom the listener answers LAN discovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Advert {
    pub addr: SocketAddr,
    /// The NetherNet id its answers come from; signals for the server go to it.
    pub network_id: u64,
}

impl Listener {
    /// The game port: UDP for RakNet and WebRTC, TCP for NetherNet signaling.
    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn lan(&self) -> Option<Advert> {
        self.lan
    }

    /// Waits for the driver to finish its goodbyes, after the game dropped its channels.
    pub async fn closed(self) {
        let _ = self.task.await;
    }
}

/// Binds `addr` and starts serving on the current tokio runtime. `keys` verify online logins
/// ([`crate::fetch_signing_keys`]); offline servers pass `SigningKeys::empty()`.
pub async fn listen(addr: SocketAddr, cfg: NetConfig, keys: SigningKeys) -> io::Result<(Listener, Outbox, Inbox)> {
    let (listener, outbox, mut inboxes) = listen_shards(addr, cfg, keys, 1, 0).await?;
    Ok((listener, outbox, inboxes.remove(0)))
}

/// [`listen`] for a game of several shards: one inbox each, new players join shard `entry`, and
/// [`Outbox::route`] moves a connection between them. The task ends when every inbox is dropped.
pub async fn listen_shards(addr: SocketAddr, cfg: NetConfig, keys: SigningKeys, shards: usize, entry: usize) -> io::Result<(Listener, Outbox, Vec<Inbox>)> {
    let (socket, tcp) = bind(addr, cfg.nethernet.is_some()).await?;
    let addr = socket.local_addr()?;
    let guid = rand_core::OsRng.next_u64();
    let mut raknet = ServerConfig::new(guid, cfg.motd.pong(guid, 0, cfg.max_players, addr.port()));
    raknet.max_peers = cfg.max_players + HANDSHAKE_SLACK;
    let (senders, inboxes) = (0..shards.max(1)).map(|_| mpsc::unbounded_channel()).unzip();
    let host = cfg.nethernet.as_ref().map(|n| Host::new(n.host_key.clone(), HostConfig { max_peers: raknet.max_peers, ..HostConfig::default() }));
    let lan = cfg.nethernet.as_ref().and_then(|n| n.lan.as_ref()).map(|lan| Lan::bind(lan.port)).transpose()?;
    let advert = lan.as_ref().map(|lan| Ok::<_, io::Error>(Advert { addr: lan.socket.local_addr()?, network_id: lan.id() })).transpose()?;
    let signaling = tcp.map(|tcp| {
        let (requests, signaling) = mpsc::unbounded_channel();
        tokio::spawn(crate::http::serve(tcp, requests));
        signaling
    });
    let (commands, outbound) = mpsc::unbounded_channel();
    let congestion = Congestion::default();
    let driver = Driver {
        socket,
        links: Links { raknet: Server::new(raknet, Instant::now()), nethernet: host },
        signaling,
        lan,
        public_ip: cfg.nethernet.as_ref().and_then(|n| n.public_ip),
        motd: cfg.motd.clone(),
        guid,
        max_players: cfg.max_players,
        sessions: Sessions::new(cfg, Verifier::new(keys), Router::new(senders, entry), congestion.clone()),
        outbound,
    };
    let task = tokio::spawn(driver.run());
    Ok((Listener { addr, lan: advert, task }, Outbox { commands, congestion }, inboxes))
}

/// The game's UDP socket and, with NetherNet, a TCP listener on the same port number.
async fn bind(addr: SocketAddr, with_tcp: bool) -> io::Result<(UdpSocket, Option<TcpListener>)> {
    if !with_tcp {
        return Ok((UdpSocket::bind(addr).await?, None));
    }
    let mut attempts = if addr.port() == 0 { PAIR_ATTEMPTS } else { 1 };
    loop {
        let socket = UdpSocket::bind(addr).await?;
        attempts -= 1;
        match TcpListener::bind(socket.local_addr()?).await {
            Ok(tcp) => return Ok((socket, Some(tcp))),
            Err(error) if attempts == 0 => return Err(error),
            Err(_) => {}
        }
    }
}

/// The NetherNet host key kept in `path`, made on first use. See [`crate::NetherNet::host_key`].
pub fn host_key(path: &Path) -> io::Result<SigningKey> {
    if let Ok(text) = std::fs::read_to_string(path) {
        let bytes: Option<Vec<u8>> = text.trim().as_bytes().chunks(2).map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()).collect();
        return bytes
            .and_then(|bytes| SigningKey::from_slice(&bytes).ok())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("{} is not a hex P-384 key", path.display())));
    }
    let key = SigningKey::random(&mut rand_core::OsRng);
    let hex: String = key.to_bytes().iter().map(|byte| format!("{byte:02x}")).collect();
    std::fs::write(path, hex)?;
    Ok(key)
}
