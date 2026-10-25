//! The network side of a sapling server: one tokio task per game socket drives acacia-raknet's
//! `Server` and, when enabled, a NetherNet host on the same socket, runs each connection's login
//! and hands verified players to the game through an inbox channel. The game answers through an
//! [`Outbox`], which never blocks. See docs/DESIGN.md "Inbound packets", "Outbound", "NetherNet".

pub mod access;
mod admission;
mod config;
mod congestion;
mod driver;
mod event;
mod http;
#[cfg(feature = "online")]
mod keys;
mod lan;
mod limits;
mod links;
mod listener;
mod players;
mod router;
mod sessions;

pub use acacia_auth::login::verify::{Identity, SigningKeys, VerifiedLogin};
pub use acacia_proto as proto;
pub use access::AccessLists;
pub use admission::Admission;
pub use config::{Compression, Lan, Motd, NetConfig, NetherNet};
pub use congestion::Congestion;
pub use event::{ConnId, Inbox, Loopback, NetEvent, Outbound, Outbox, ShardsLoopback, loopback, loopback_shards};
#[cfg(feature = "online")]
pub use keys::{KeyError, fetch_signing_keys};
pub use limits::PacketLimits;
pub use listener::{Advert, Listener, host_key, listen, listen_shards};
pub use router::Router;

use acacia_proto::Packet;
use bytes::{Bytes, BytesMut};

/// A packet with its header, as [`Outbox::send`] takes it.
pub fn encode<T: Packet>(packet: &T) -> Bytes {
    let mut buf = BytesMut::new();
    acacia_proto::encode_packet(packet, &mut buf);
    buf.freeze()
}
