//! What carries players' batches to the server. RakNet and NetherNet are built in
//! (`transports/`); an embedder adds others with `ServerBuilder::transport`, as dragonfly's
//! `ListenNetwork` takes any gophertunnel `Network`. Logins, limits and kicks run above it, the same
//! for every transport. See docs/DESIGN.md "Transports".

use std::net::SocketAddr;
use std::task::{Context, Poll};
use std::time::Instant;

use bytes::Bytes;

/// A transport's name for one of its connections, unique among its open ones.
pub type ConnKey = u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportEvent {
    /// A client is connected; its login starts.
    Connected(ConnKey),
    /// One batch from the client.
    Message(ConnKey, Bytes),
    /// The connection ended, from either side; the key may be used again after this.
    Disconnected(ConnKey, String),
}

/// How batches are written on a transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Framing {
    /// RakNet's: a `0xfe` header, encrypted after the login handshake.
    #[default]
    Batched,
    /// NetherNet's: no header and never encrypted, the transport being secure itself.
    Plain,
}

/// A source of player connections, driven by the listener's task. Sans-IO by default: the built-in
/// transports are fed the game socket's datagrams; one with sockets or channels of its own polls them
/// in [`Transport::poll_ready`]. Methods must not block.
pub trait Transport: Send + 'static {
    /// For logs.
    fn name(&self) -> &str;

    fn framing(&self) -> Framing {
        Framing::Batched
    }

    /// Does the transport's own I/O, registering `cx` to be woken for more. `Ready` when events may
    /// be waiting. The default never is: it suits transports fed by [`Transport::handle_datagram`].
    fn poll_ready(&mut self, _cx: &mut Context<'_>, _now: Instant) -> Poll<()> {
        Poll::Pending
    }

    /// A datagram on the game's UDP socket; true if it is this transport's. Transports are offered
    /// it in turn, RakNet last.
    fn handle_datagram(&mut self, _now: Instant, _from: SocketAddr, _data: &[u8]) -> bool {
        false
    }

    /// The next datagram for the game's UDP socket.
    fn poll_transmit(&mut self, _now: Instant) -> Option<(SocketAddr, Bytes)> {
        None
    }

    fn poll_timeout(&self) -> Option<Instant> {
        None
    }

    fn handle_timeout(&mut self, _now: Instant) {}

    fn poll_event(&mut self) -> Option<TransportEvent>;

    /// Queues one batch; dropped if the connection is gone.
    fn send(&mut self, conn: ConnKey, batch: Bytes, now: Instant);

    /// Flushes what is queued, then ends the connection; [`TransportEvent::Disconnected`] follows.
    fn close(&mut self, conn: ConnKey, now: Instant);

    /// Bytes not yet sent to the client, for the backlog limit and congestion.
    fn queued_bytes(&self, _conn: ConnKey) -> usize {
        0
    }

    /// The client's address: admission counts and bans go by its IP.
    fn remote_addr(&self, conn: ConnKey) -> Option<SocketAddr>;
}
