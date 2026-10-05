//! Both transports behind one face: a session talks to a [`Link`], whichever carries it.

use std::fmt;
use std::net::{Ipv4Addr, SocketAddr};
use std::time::Instant;

use acacia_nethernet::host::{Host, HostEvent, PeerId};
use acacia_raknet::{Reliability, Server, ServerEvent};
use bytes::Bytes;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Link {
    RakNet(SocketAddr),
    NetherNet(PeerId),
}

impl fmt::Display for Link {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RakNet(addr) => write!(f, "raknet {addr}"),
            Self::NetherNet(peer) => write!(f, "nethernet #{}", peer.0),
        }
    }
}

pub(crate) enum LinkEvent {
    /// The transport is up; the login starts.
    Connected(Link),
    /// One game message (a batch).
    Message(Link, Bytes),
    Disconnected(Link, String),
}

/// Why an offer got no answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// Its `a=identity` is missing, broken or (online mode) not the auth service's.
    Identity,
    Full,
    /// The SDP could not be answered.
    Offer,
}

impl Refusal {
    /// The NetherNet error code for it (`acacia_nethernet::signaling_error_name`).
    pub fn code(self) -> u32 {
        match self {
            Self::Identity => 37,
            Self::Full => 16,
            Self::Offer => 13,
        }
    }
}

pub(crate) struct Links {
    pub raknet: Server,
    pub nethernet: Option<Host>,
}

impl Links {
    /// A datagram from the game socket: NetherNet claims what is its own, RakNet gets the rest.
    pub fn handle_datagram(&mut self, now: Instant, from: SocketAddr, data: &[u8]) {
        if !self.nethernet.as_mut().is_some_and(|host| host.handle_datagram(now, from, data)) {
            self.raknet.handle_datagram(now, from, Bytes::copy_from_slice(data));
        }
    }

    pub fn poll_event(&mut self) -> Option<LinkEvent> {
        if let Some(event) = self.raknet.poll_event() {
            return Some(match event {
                ServerEvent::Connected { addr, .. } => LinkEvent::Connected(Link::RakNet(addr)),
                ServerEvent::Message(addr, msg) => LinkEvent::Message(Link::RakNet(addr), msg),
                ServerEvent::Disconnected(addr, reason) => LinkEvent::Disconnected(Link::RakNet(addr), format!("{reason:?}")),
            });
        }
        Some(match self.nethernet.as_mut()?.poll_event()? {
            HostEvent::Open(peer) => LinkEvent::Connected(Link::NetherNet(peer)),
            HostEvent::Message(peer, msg) => LinkEvent::Message(Link::NetherNet(peer), msg),
            HostEvent::Closed(peer, reason) => LinkEvent::Disconnected(Link::NetherNet(peer), reason),
        })
    }

    pub fn poll_transmit(&mut self, now: Instant) -> Option<(SocketAddr, Bytes)> {
        self.raknet.poll_transmit(now).or_else(|| self.nethernet.as_mut()?.poll_transmit().map(|(to, data)| (to, Bytes::from(data))))
    }

    pub fn poll_timeout(&self) -> Option<Instant> {
        self.raknet.poll_timeout().into_iter().chain(self.nethernet.as_ref().and_then(Host::poll_timeout)).min()
    }

    pub fn handle_timeout(&mut self, now: Instant) {
        self.raknet.handle_timeout(now);
        if let Some(host) = self.nethernet.as_mut() {
            host.handle_timeout(now);
        }
    }

    pub fn send(&mut self, link: Link, msg: Bytes, now: Instant) {
        match (link, self.nethernet.as_mut()) {
            (Link::RakNet(addr), _) => {
                self.raknet.send(addr, msg, Reliability::ReliableOrdered);
            }
            (Link::NetherNet(peer), Some(host)) => host.send(peer, msg, now),
            (Link::NetherNet(_), None) => {}
        }
    }

    /// Flushes what is queued for the link, then closes it.
    pub fn close(&mut self, link: Link, now: Instant) {
        match (link, self.nethernet.as_mut()) {
            (Link::RakNet(addr), _) => self.raknet.close(addr, now),
            (Link::NetherNet(peer), Some(host)) => host.close(peer, now),
            (Link::NetherNet(_), None) => {}
        }
    }

    /// Bytes waiting to leave for the link.
    pub fn queued_bytes(&self, link: Link) -> usize {
        match link {
            Link::RakNet(addr) => self.raknet.stats(addr).map(|s| s.queued_bytes),
            Link::NetherNet(peer) => self.nethernet.as_ref().and_then(|host| host.queued_bytes(peer)),
        }
        .unwrap_or(0)
    }

    pub fn remote_addr(&self, link: Link) -> SocketAddr {
        match link {
            Link::RakNet(addr) => Some(addr),
            Link::NetherNet(peer) => self.nethernet.as_ref().and_then(|host| host.remote_addr(peer)),
        }
        .unwrap_or((Ipv4Addr::UNSPECIFIED, 0).into())
    }
}
