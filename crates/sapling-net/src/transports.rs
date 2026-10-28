//! The built-in [`Transport`]s: RakNet, keyed by client address, and NetherNet, keyed by peer.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Instant;

use acacia_nethernet::host::{Host, HostEvent, PeerId};
use acacia_raknet::{Reliability, Server, ServerEvent};
use bytes::Bytes;

use crate::transport::{ConnKey, Framing, Transport, TransportEvent};

pub(crate) struct RakNetTransport {
    pub server: Server,
    keys: HashMap<SocketAddr, ConnKey>,
    addrs: HashMap<ConnKey, SocketAddr>,
    next: ConnKey,
}

impl RakNetTransport {
    pub fn new(server: Server) -> Self {
        Self { server, keys: HashMap::new(), addrs: HashMap::new(), next: 1 }
    }
}

impl Transport for RakNetTransport {
    fn name(&self) -> &str {
        "raknet"
    }

    fn handle_datagram(&mut self, now: Instant, from: SocketAddr, data: &[u8]) -> bool {
        self.server.handle_datagram(now, from, Bytes::copy_from_slice(data));
        true
    }

    fn poll_transmit(&mut self, now: Instant) -> Option<(SocketAddr, Bytes)> {
        self.server.poll_transmit(now)
    }

    fn poll_timeout(&self) -> Option<Instant> {
        self.server.poll_timeout()
    }

    fn handle_timeout(&mut self, now: Instant) {
        self.server.handle_timeout(now);
    }

    fn poll_event(&mut self) -> Option<TransportEvent> {
        loop {
            return Some(match self.server.poll_event()? {
                ServerEvent::Connected { addr, .. } => {
                    let key = self.next;
                    self.next += 1;
                    self.keys.insert(addr, key);
                    self.addrs.insert(key, addr);
                    TransportEvent::Connected(key)
                }
                ServerEvent::Message(addr, msg) => match self.keys.get(&addr) {
                    Some(&key) => TransportEvent::Message(key, msg),
                    None => continue,
                },
                ServerEvent::Disconnected(addr, reason) => match self.keys.remove(&addr) {
                    Some(key) => {
                        self.addrs.remove(&key);
                        TransportEvent::Disconnected(key, format!("{reason:?}"))
                    }
                    None => continue,
                },
            });
        }
    }

    fn send(&mut self, conn: ConnKey, batch: Bytes, _now: Instant) {
        if let Some(&addr) = self.addrs.get(&conn) {
            self.server.send(addr, batch, Reliability::ReliableOrdered);
        }
    }

    fn close(&mut self, conn: ConnKey, now: Instant) {
        if let Some(&addr) = self.addrs.get(&conn) {
            self.server.close(addr, now);
        }
    }

    fn queued_bytes(&self, conn: ConnKey) -> usize {
        self.addrs.get(&conn).and_then(|addr| self.server.stats(*addr)).map_or(0, |stats| stats.queued_bytes)
    }

    fn remote_addr(&self, conn: ConnKey) -> Option<SocketAddr> {
        self.addrs.get(&conn).copied()
    }
}

pub(crate) struct NetherNetTransport {
    pub host: Host,
}

impl Transport for NetherNetTransport {
    fn name(&self) -> &str {
        "nethernet"
    }

    fn framing(&self) -> Framing {
        Framing::Plain
    }

    fn handle_datagram(&mut self, now: Instant, from: SocketAddr, data: &[u8]) -> bool {
        self.host.handle_datagram(now, from, data)
    }

    fn poll_transmit(&mut self, _now: Instant) -> Option<(SocketAddr, Bytes)> {
        self.host.poll_transmit().map(|(to, data)| (to, Bytes::from(data)))
    }

    fn poll_timeout(&self) -> Option<Instant> {
        self.host.poll_timeout()
    }

    fn handle_timeout(&mut self, now: Instant) {
        self.host.handle_timeout(now);
    }

    fn poll_event(&mut self) -> Option<TransportEvent> {
        Some(match self.host.poll_event()? {
            HostEvent::Open(peer) => TransportEvent::Connected(peer.0),
            HostEvent::Message(peer, msg) => TransportEvent::Message(peer.0, msg),
            HostEvent::Closed(peer, reason) => TransportEvent::Disconnected(peer.0, reason),
        })
    }

    fn send(&mut self, conn: ConnKey, batch: Bytes, now: Instant) {
        self.host.send(PeerId(conn), batch, now);
    }

    fn close(&mut self, conn: ConnKey, now: Instant) {
        self.host.close(PeerId(conn), now);
    }

    fn queued_bytes(&self, conn: ConnKey) -> usize {
        self.host.queued_bytes(PeerId(conn)).unwrap_or(0)
    }

    fn remote_addr(&self, conn: ConnKey) -> Option<SocketAddr> {
        self.host.remote_addr(PeerId(conn))
    }
}
