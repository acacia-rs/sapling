use std::net::SocketAddr;

use acacia_auth::login::verify::VerifiedLogin;
use acacia_proto::RawPacket;
use bytes::Bytes;
use tokio::sync::mpsc;

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
}

#[derive(Debug)]
pub enum Outbound {
    /// Packets (each with its header) to send as one batch.
    Send { conn: ConnId, packets: Vec<Bytes> },
    /// Shows `message` on the client's disconnect screen and closes the connection.
    Kick { conn: ConnId, message: String },
}

/// The game's handle for sending. Sends queue without blocking; a connection that has gone is
/// skipped silently (its `Left` is already in the inbox). Dropping every clone stops the listener.
#[derive(Debug, Clone)]
pub struct Outbox(pub(crate) mpsc::UnboundedSender<Outbound>);

impl Outbox {
    pub fn send(&self, conn: ConnId, packets: Vec<Bytes>) {
        if !packets.is_empty() {
            let _ = self.0.send(Outbound::Send { conn, packets });
        }
    }

    pub fn kick(&self, conn: ConnId, message: impl Into<String>) {
        let _ = self.0.send(Outbound::Kick { conn, message: message.into() });
    }
}

/// Where the game reads [`NetEvent`]s; `try_recv` works from any thread.
pub type Inbox = mpsc::UnboundedReceiver<NetEvent>;

/// The network's ends of a game's channels, for driving a shard without a listener (tests).
pub struct Loopback {
    pub events: mpsc::UnboundedSender<NetEvent>,
    pub sent: mpsc::UnboundedReceiver<Outbound>,
}

pub fn loopback() -> (Loopback, Outbox, Inbox) {
    let (events, inbox) = mpsc::unbounded_channel();
    let (outbox, sent) = mpsc::unbounded_channel();
    (Loopback { events, sent }, Outbox(outbox), inbox)
}
