//! LAN visibility: answers discovery broadcasts and carries a join's signals in discovery Message
//! packets. Wire spec: acacia `docs/research/nethernet-signaling.md` §1.

use std::collections::HashMap;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket as StdUdpSocket};
use std::time::Instant;

use acacia_nethernet::host::PeerId;
use acacia_nethernet::lan::{LanPacket, ServerData};
use acacia_nethernet::{Signal, SignalKind};
use rand_core::RngCore;
use socket2::{Domain, Protocol, Socket, Type};
use tokio::net::UdpSocket;

use crate::links::Links;
use crate::sessions::Sessions;

pub(crate) struct Lan {
    pub socket: UdpSocket,
    /// Our NetherNet id: the sender of our packets, the recipient of signals meant for us.
    id: u64,
    /// Joins by (client id, connection id), for the candidates that follow an offer.
    joins: HashMap<(u64, u64), PeerId>,
}

impl Lan {
    /// Binds the discovery port on every IPv4 interface. The port is shared, since a game running
    /// on this machine holds it too.
    pub fn bind(port: u16) -> io::Result<Self> {
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
        socket.set_reuse_address(true)?;
        socket.set_broadcast(true)?;
        socket.set_nonblocking(true)?;
        socket.bind(&SocketAddr::from((Ipv4Addr::UNSPECIFIED, port)).into())?;
        Ok(Self { socket: UdpSocket::from_std(socket.into())?, id: rand_core::OsRng.next_u64(), joins: HashMap::new() })
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    /// Handles one discovery datagram and returns the datagrams to send back to `from`.
    /// `game_port` is the UDP port WebRTC runs on (NetherNet's own, if it has one).
    pub fn handle(
        &mut self,
        data: &[u8],
        from: SocketAddr,
        game_port: u16,
        advert: impl FnOnce() -> ServerData,
        sessions: &mut Sessions,
        links: &mut Links,
        now: Instant,
    ) -> Vec<Vec<u8>> {
        let (packet, sender) = match LanPacket::decode(data) {
            Ok(decoded) => decoded,
            Err(error) => {
                tracing::trace!(%from, %error, "not a LAN packet");
                return Vec::new();
            }
        };
        if sender == self.id {
            return Vec::new();
        }
        let signals = match packet {
            LanPacket::Request => return vec![LanPacket::Response(advert().encode()).encode(self.id)],
            LanPacket::Message { recipient, data } if recipient == self.id => match Signal::parse(&data) {
                Ok(signal) => self.on_signal(signal, sender, from, game_port, sessions, links, now),
                Err(error) => {
                    tracing::debug!(%from, %error, "bad LAN signal");
                    Vec::new()
                }
            },
            _ => Vec::new(),
        };
        signals.into_iter().map(|signal| LanPacket::Message { recipient: sender, data: signal.to_string() }.encode(self.id)).collect()
    }

    #[allow(clippy::too_many_arguments)]
    fn on_signal(&mut self, signal: Signal, sender: u64, from: SocketAddr, game_port: u16, sessions: &mut Sessions, links: &mut Links, now: Instant) -> Vec<Signal> {
        let join = (sender, signal.connection_id);
        match signal.kind {
            SignalKind::ConnectRequest => {
                let local = SocketAddr::new(local_ip_towards(from), game_port);
                match sessions.admit_offer(links, &signal.data, local, from, true, now) {
                    Ok(answer) => {
                        self.joins.insert(join, answer.peer);
                        let candidates = answer.candidates.into_iter().map(|line| Signal::new(SignalKind::CandidateAdd, join.1, line));
                        std::iter::once(Signal::new(SignalKind::ConnectResponse, join.1, answer.sdp)).chain(candidates).collect()
                    }
                    Err(refusal) => vec![Signal::error(join.1, refusal.code())],
                }
            }
            SignalKind::CandidateAdd => {
                if let (Some(&peer), Some(nethernet)) = (self.joins.get(&join), links.nethernet.as_mut())
                    && let Err(error) = nethernet.host.add_remote_candidate(peer, &signal.data, now)
                {
                    tracing::debug!(%from, %error, "bad LAN candidate");
                }
                Vec::new()
            }
            SignalKind::ConnectError => {
                if let (Some(peer), Some(nethernet)) = (self.joins.remove(&join), links.nethernet.as_mut()) {
                    nethernet.host.close(peer, now);
                }
                Vec::new()
            }
            SignalKind::ConnectResponse => Vec::new(),
        }
    }

    /// Drops the join of a connection that ended.
    pub fn forget(&mut self, peer: PeerId) {
        self.joins.retain(|_, joined| *joined != peer);
    }
}

/// Our address on the interface that routes to `peer`: the candidate it can reach.
fn local_ip_towards(peer: SocketAddr) -> IpAddr {
    let probe = || {
        let socket = StdUdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
        socket.connect(peer)?;
        socket.local_addr()
    };
    probe().map_or(IpAddr::V4(Ipv4Addr::LOCALHOST), |addr| addr.ip())
}
