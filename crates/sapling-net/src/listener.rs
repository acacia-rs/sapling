use std::io;
use std::net::SocketAddr;
use std::time::Instant;

use acacia_auth::login::verify::{SigningKeys, Verifier};
use acacia_raknet::{Server, ServerConfig};
use rand_core::RngCore;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::config::NetConfig;
use crate::driver::Driver;
use crate::event::{Inbox, Outbox};
use crate::router::Router;
use crate::sessions::Sessions;

/// RakNet peers allowed beyond `max_players`, so a full server can still tell newcomers it is full.
const HANDSHAKE_SLACK: usize = 32;

/// A listening socket and the task driving it. The task ends when the game drops the [`Outbox`]
/// or the [`Inbox`]; players are then told the server closed.
pub struct Listener {
    addr: SocketAddr,
    task: JoinHandle<()>,
}

impl Listener {
    pub fn local_addr(&self) -> SocketAddr {
        self.addr
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
    let socket = UdpSocket::bind(addr).await?;
    let addr = socket.local_addr()?;
    let guid = rand_core::OsRng.next_u64();
    let mut raknet = ServerConfig::new(guid, cfg.motd.pong(guid, 0, cfg.max_players, addr.port()));
    raknet.max_peers = cfg.max_players + HANDSHAKE_SLACK;
    let (senders, inboxes) = (0..shards.max(1)).map(|_| mpsc::unbounded_channel()).unzip();
    let (outbox, outbound) = mpsc::unbounded_channel();
    let driver = Driver {
        socket,
        server: Server::new(raknet, Instant::now()),
        motd: cfg.motd.clone(),
        guid,
        max_players: cfg.max_players,
        sessions: Sessions::new(cfg, Verifier::new(keys), Router::new(senders, entry)),
        outbound,
    };
    let task = tokio::spawn(driver.run());
    Ok((Listener { addr, task }, Outbox(outbox), inboxes))
}
