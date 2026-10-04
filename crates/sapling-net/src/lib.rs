//! The network side of a sapling server: one tokio task per UDP socket drives acacia-raknet's
//! `Server`, runs each connection's login and hands verified players to the game through an inbox
//! channel. The game answers through an [`Outbox`], which never blocks. See docs/DESIGN.md
//! "Inbound packets" and "Outbound".

mod config;
mod driver;
mod event;
#[cfg(feature = "online")]
mod keys;
mod listener;
mod router;
mod sessions;

pub use acacia_auth::login::verify::{Identity, SigningKeys, VerifiedLogin};
pub use acacia_proto as proto;
pub use config::{Motd, NetConfig};
pub use event::{ConnId, Inbox, Loopback, NetEvent, Outbound, Outbox, ShardsLoopback, loopback, loopback_shards};
#[cfg(feature = "online")]
pub use keys::{KeyError, fetch_signing_keys};
pub use listener::{Listener, listen, listen_shards};
pub use router::Router;

use acacia_proto::Packet;
use bytes::{Bytes, BytesMut};

/// A packet with its header, as [`Outbox::send`] takes it.
pub fn encode<T: Packet>(packet: &T) -> Bytes {
    let mut buf = BytesMut::new();
    acacia_proto::encode_packet(packet, &mut buf);
    buf.freeze()
}
