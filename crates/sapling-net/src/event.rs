use std::net::SocketAddr;

use acacia_auth::login::verify::VerifiedLogin;
use acacia_proto::RawPacket;
use acacia_proto::types::DisconnectFailReason;
use bytes::Bytes;
use tokio::sync::mpsc;

use crate::congestion::Congestion;
use crate::router::Router;

/// One logged-in connection. Never reused within a process, so a late message for a player who
/// left cannot reach whoever connects next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConnId(pub(crate) u64);

impl ConnId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

/// What the network tells the game. Per connection: `Joined`, then `Packet`s, then exactly one
/// `Left`, however the connection ended (the game's own kick included).
#[derive(Debug)]
pub enum NetEvent {
    /// Login succeeded (PlayStatus(LoginSuccess) is on its way) and the identity is reserved.
    Joined { conn: ConnId, addr: SocketAddr, login: Box<VerifiedLogin> },
    /// Header parsed, body left to decode on demand.
    Packet { conn: ConnId, packet: RawPacket },
    Left { conn: ConnId, reason: String },
    /// The answer to [`Outbox::route`]: nothing more of `conn` comes to this shard.
    Rerouted { conn: ConnId },
}

#[derive(Debug)]
pub enum Outbound {
    /// Packets (each with its header) to send as one batch.
    Send { conn: ConnId, packets: Vec<Bytes> },
    /// Shows `message` on the client's disconnect screen and closes the connection.
    Kick { conn: ConnId, reason: DisconnectFailReason, message: String },
    /// Hands the connection's events to shard `to` from here on ([`crate::Router::reroute`]).
    Route { conn: ConnId, from: usize, to: usize },
}

/// The game's handle for sending. Sends queue without blocking; a connection that has gone is
/// skipped silently (its `Left` is already in the inbox). Dropping every clone stops the listener.
#[derive(Debug, Clone)]
pub struct Outbox {
    pub(crate) commands: mpsc::UnboundedSender<Outbound>,
    pub(crate) congestion: Congestion,
}

impl Outbox {
    pub fn send(&self, conn: ConnId, packets: Vec<Bytes>) {
        if !packets.is_empty() {
            let _ = self.commands.send(Outbound::Send { conn, packets });
        }
    }

    pub fn kick(&self, conn: ConnId, message: impl Into<String>) {
        self.kick_for(conn, DisconnectFailReason::Kicked, message);
    }

    /// [`Outbox::kick`] under another reason code than `Kicked`.
    pub fn kick_for(&self, conn: ConnId, reason: DisconnectFailReason, message: impl Into<String>) {
        let _ = self.commands.send(Outbound::Kick { conn, reason, message: message.into() });
    }

    /// Moves a connection to another shard; shard `from` gets [`NetEvent::Rerouted`] when done.
    pub fn route(&self, conn: ConnId, from: usize, to: usize) {
        let _ = self.commands.send(Outbound::Route { conn, from, to });
    }

    /// Whether the connection has more unsent than it is draining: what can wait should.
    pub fn is_congested(&self, conn: ConnId) -> bool {
        self.congestion.contains(conn)
    }
}

/// Where the game reads [`NetEvent`]s; `try_recv` works from any thread.
pub type Inbox = mpsc::UnboundedReceiver<NetEvent>;

/// The network's ends of a game's channels, for driving a shard without a listener (tests).
pub struct Loopback {
    pub events: mpsc::UnboundedSender<NetEvent>,
    pub sent: mpsc::UnboundedReceiver<Outbound>,
    pub congestion: Congestion,
}

pub fn loopback() -> (Loopback, Outbox, Inbox) {
    let (events, inbox) = mpsc::unbounded_channel();
    let (commands, sent) = mpsc::unbounded_channel();
    let congestion = Congestion::default();
    (Loopback { events, sent, congestion: congestion.clone() }, Outbox { commands, congestion }, inbox)
}

/// The network's side of several shards, for driving them without a listener (tests).
pub struct ShardsLoopback {
    pub router: Router,
    pub congestion: Congestion,
    sent: mpsc::UnboundedReceiver<Outbound>,
}

impl ShardsLoopback {
    /// What the shards sent since the last call; their reroutes are carried out, in order.
    pub fn pump(&mut self) -> Vec<Outbound> {
        let mut sent = Vec::new();
        while let Ok(command) = self.sent.try_recv() {
            match command {
                Outbound::Route { conn, from, to } => drop(self.router.reroute(conn, from, to)),
                other => sent.push(other),
            }
        }
        sent
    }
}

/// `shards` inboxes behind one router whose new connections join shard `entry`.
pub fn loopback_shards(shards: usize, entry: usize) -> (ShardsLoopback, Outbox, Vec<Inbox>) {
    let (senders, inboxes) = (0..shards).map(|_| mpsc::unbounded_channel()).unzip();
    let (commands, sent) = mpsc::unbounded_channel();
    let congestion = Congestion::default();
    (ShardsLoopback { router: Router::new(senders, entry), congestion: congestion.clone(), sent }, Outbox { commands, congestion }, inboxes)
}
