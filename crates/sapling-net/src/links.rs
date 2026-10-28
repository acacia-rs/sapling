//! Every transport behind one face: a session talks to a [`Link`], whichever carries it.

use std::fmt;
use std::net::{Ipv4Addr, SocketAddr};
use std::task::{Context, Poll};
use std::time::Instant;

use bytes::Bytes;

use crate::sockets::Via;
use crate::transport::{ConnKey, Framing, Transport, TransportEvent};
use crate::transports::{NetherNetTransport, RakNetTransport};

pub(crate) const RAKNET: u16 = 0;
pub(crate) const NETHERNET: u16 = 1;
/// Embedders' transports are numbered from here, in the order given.
const CUSTOM: u16 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Link {
    pub transport: u16,
    pub conn: ConnKey,
}

impl fmt::Display for Link {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.transport {
            RAKNET => write!(f, "raknet #{}", self.conn),
            NETHERNET => write!(f, "nethernet #{}", self.conn),
            n => write!(f, "transport {n} #{}", self.conn),
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
    pub raknet: RakNetTransport,
    pub nethernet: Option<NetherNetTransport>,
    pub custom: Vec<Box<dyn Transport>>,
}

impl Links {
    fn get(&self, transport: u16) -> Option<&dyn Transport> {
        match transport {
            RAKNET => Some(&self.raknet),
            NETHERNET => self.nethernet.as_ref().map(|n| n as &dyn Transport),
            n => self.custom.get(usize::from(n - CUSTOM)).map(|t| &**t),
        }
    }

    fn get_mut(&mut self, transport: u16) -> Option<&mut dyn Transport> {
        match transport {
            RAKNET => Some(&mut self.raknet),
            NETHERNET => self.nethernet.as_mut().map(|n| n as &mut dyn Transport),
            n => self.custom.get_mut(usize::from(n - CUSTOM)).map(|t| &mut **t),
        }
    }

    /// Each transport with its number, in the order they are offered a datagram: RakNet takes
    /// whatever is left, so it comes last.
    fn all_mut(&mut self) -> impl Iterator<Item = (u16, &mut dyn Transport)> {
        let nethernet = self.nethernet.as_mut().map(|n| (NETHERNET, n as &mut dyn Transport));
        let custom = self.custom.iter_mut().zip(CUSTOM..).map(|(t, n)| (n, &mut **t));
        nethernet.into_iter().chain(custom).chain([(RAKNET, &mut self.raknet as &mut dyn Transport)])
    }

    fn all(&self) -> impl Iterator<Item = &dyn Transport> {
        let nethernet = self.nethernet.as_ref().map(|n| n as &dyn Transport);
        nethernet.into_iter().chain(self.custom.iter().map(|t| &**t)).chain([&self.raknet as &dyn Transport])
    }

    /// A datagram from one of our UDP sockets; NetherNet's own socket feeds NetherNet alone.
    pub fn handle_datagram(&mut self, now: Instant, from: SocketAddr, data: &[u8], via: Via) {
        if via == Via::NetherNet {
            if let Some(nethernet) = self.nethernet.as_mut() {
                nethernet.handle_datagram(now, from, data);
            }
            return;
        }
        for (_, transport) in self.all_mut() {
            if transport.handle_datagram(now, from, data) {
                return;
            }
        }
    }

    pub fn poll_event(&mut self) -> Option<LinkEvent> {
        self.all_mut().find_map(|(transport, t)| {
            Some(match t.poll_event()? {
                TransportEvent::Connected(conn) => LinkEvent::Connected(Link { transport, conn }),
                TransportEvent::Message(conn, msg) => LinkEvent::Message(Link { transport, conn }, msg),
                TransportEvent::Disconnected(conn, reason) => LinkEvent::Disconnected(Link { transport, conn }, reason),
            })
        })
    }

    /// The next datagram, and the socket it leaves by.
    pub fn poll_transmit(&mut self, now: Instant) -> Option<(Via, SocketAddr, Bytes)> {
        self.all_mut().find_map(|(transport, t)| {
            let via = if transport == NETHERNET { Via::NetherNet } else { Via::Game };
            t.poll_transmit(now).map(|(to, data)| (via, to, data))
        })
    }

    pub fn poll_timeout(&self) -> Option<Instant> {
        self.all().filter_map(|t| t.poll_timeout()).min()
    }

    pub fn handle_timeout(&mut self, now: Instant) {
        for (_, transport) in self.all_mut() {
            transport.handle_timeout(now);
        }
    }

    /// Ready when a transport with I/O of its own may have events.
    pub fn poll_ready(&mut self, cx: &mut Context<'_>, now: Instant) -> Poll<()> {
        let mut ready = false;
        for transport in &mut self.custom {
            ready |= transport.poll_ready(cx, now).is_ready();
        }
        if ready { Poll::Ready(()) } else { Poll::Pending }
    }

    pub fn framing(&self, link: Link) -> Framing {
        self.get(link.transport).map_or_else(Framing::default, |t| t.framing())
    }

    pub fn send(&mut self, link: Link, msg: Bytes, now: Instant) {
        if let Some(transport) = self.get_mut(link.transport) {
            transport.send(link.conn, msg, now);
        }
    }

    /// Flushes what is queued for the link, then closes it.
    pub fn close(&mut self, link: Link, now: Instant) {
        if let Some(transport) = self.get_mut(link.transport) {
            transport.close(link.conn, now);
        }
    }

    /// Bytes waiting to leave for the link.
    pub fn queued_bytes(&self, link: Link) -> usize {
        self.get(link.transport).map_or(0, |t| t.queued_bytes(link.conn))
    }

    pub fn remote_addr(&self, link: Link) -> SocketAddr {
        self.get(link.transport).and_then(|t| t.remote_addr(link.conn)).unwrap_or((Ipv4Addr::UNSPECIFIED, 0).into())
    }
}
