//! The socket loop: datagrams and signaling in, the transports' output out, timers in between.

use std::future::pending;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use crate::config::Motd;
use crate::event::Outbound;
use crate::http::Request;
use crate::lan::Lan;
use crate::links::{Link, LinkEvent, Links};
use crate::query::{Query, Status};
use crate::sessions::Sessions;
use crate::sockets::Sockets;

/// Longer than any datagram a peer may send (the RakNet MTU tops out at 1492).
const DATAGRAM_BUF: usize = 2048;
/// A LAN Message carries a whole SDP offer with its token.
const LAN_BUF: usize = 8192;
/// Work taken per wake-up before sending replies, so a flood on one side cannot starve the other.
const BURST: usize = 256;
/// Wake-up when nothing is scheduled, to notice a game that went away.
const IDLE_WAKE: Duration = Duration::from_millis(500);
/// How often backlogs are re-read for congestion: well inside a game tick, without walking every
/// player on every datagram.
const BACKLOG_CHECK: Duration = Duration::from_millis(10);

pub(crate) struct Driver {
    pub sockets: Sockets,
    /// `None` unless the query is enabled.
    pub query: Option<Query>,
    /// Query answers waiting for the next flush.
    pub replies: Vec<(SocketAddr, Vec<u8>)>,
    pub links: Links,
    pub sessions: Sessions,
    pub outbound: mpsc::UnboundedReceiver<Outbound>,
    /// Direct-connect signaling; `None` without NetherNet.
    pub signaling: Option<mpsc::UnboundedReceiver<Request>>,
    pub lan: Option<Lan>,
    /// Advertised in answers instead of the address signaling arrived on.
    pub public_ip: Option<IpAddr>,
    pub motd: Motd,
    pub guid: u64,
    pub max_players: crate::PlayerLimit,
}

impl Driver {
    /// Runs until the game drops its outbox or inbox, then says goodbye to every player.
    pub async fn run(mut self) {
        let ports = self.sockets.ports();
        let port = ports[0];
        let mut advertised = (usize::MAX, 0);
        let mut backlogs_checked = Instant::now();
        let (mut buf, mut lan_buf) = (vec![0u8; DATAGRAM_BUF], vec![0u8; LAN_BUF]);
        while self.sessions.has_game() {
            // Events first: the replies they queue must leave in this turn, not at the next wake-up.
            let now = Instant::now();
            while let Some(event) = self.links.poll_event() {
                if let (LinkEvent::Disconnected(Link::NetherNet(peer), _), Some(lan)) = (&event, self.lan.as_mut()) {
                    lan.forget(*peer);
                }
                self.sessions.on_event(&mut self.links, event, now);
            }
            self.sessions.expire_logins(&mut self.links, now);
            self.sessions.enforce_bans(&mut self.links, now);
            if advertised != (self.sessions.player_count(), self.max_players.get()) {
                advertised = (self.sessions.player_count(), self.max_players.get());
                self.links.raknet.set_motd(self.motd.pong(self.guid, advertised.0, advertised.1, ports));
            }
            self.flush(now).await;
            if now.duration_since(backlogs_checked) >= BACKLOG_CHECK {
                backlogs_checked = now;
                self.sessions.track_backlogs(&self.links);
            }
            let timers = self.links.poll_timeout().into_iter().chain(self.sessions.next_deadline());
            let deadline = timers.min().unwrap_or(now + IDLE_WAKE);
            tokio::select! {
                received = self.sockets.recv_from(&mut buf) => {
                    let now = Instant::now();
                    let mut received = received.ok();
                    for _ in 0..BURST {
                        let Some((len, from)) = received else { break };
                        self.on_datagram(&buf[..len], from, now);
                        received = self.sockets.try_recv_from(&mut buf).ok();
                    }
                }
                command = self.outbound.recv() => {
                    let now = Instant::now();
                    let Some(command) = command else { break };
                    self.sessions.on_outbound(&mut self.links, command, now);
                    for _ in 0..BURST {
                        let Ok(command) = self.outbound.try_recv() else { break };
                        self.sessions.on_outbound(&mut self.links, command, now);
                    }
                }
                request = next_request(&mut self.signaling) => self.on_request(request, port),
                received = lan_recv(self.lan.as_ref(), &mut lan_buf) => {
                    if let Ok((len, from)) = received {
                        self.on_lan_datagram(&lan_buf[..len], from, port).await;
                    }
                }
                _ = tokio::time::sleep_until(deadline.into()) => self.links.handle_timeout(Instant::now()),
            }
        }
        let now = Instant::now();
        self.sessions.close_all(&mut self.links, now);
        self.flush(now).await;
    }

    fn on_request(&mut self, request: Request, port: u16) {
        match request {
            Request::Status(reply) => {
                let _ = reply.send(self.motd.status(self.sessions.player_count(), self.max_players.get()));
            }
            Request::Join { offer, local, reply } => {
                let local = SocketAddr::new(self.public_ip.unwrap_or(local), port);
                let answer = self.sessions.admit_offer(&mut self.links, &offer, local, false, Instant::now());
                let _ = reply.send(answer.map(|answer| answer.sdp));
            }
        }
    }

    async fn on_lan_datagram(&mut self, data: &[u8], from: SocketAddr, port: u16) {
        let Some(lan) = self.lan.as_mut() else { return };
        let (players, online) = (self.sessions.player_count(), self.sessions.online_mode());
        let advert = || self.motd.server_data(players, self.max_players.get(), online);
        for reply in lan.handle(data, from, port, advert, &mut self.sessions, &mut self.links, Instant::now()) {
            let _ = lan.socket.send_to(&reply, from).await;
        }
    }

    fn on_datagram(&mut self, data: &[u8], from: SocketAddr, now: Instant) {
        let Some(query) = self.query.as_mut().filter(|_| Query::is_query(data)) else {
            self.links.handle_datagram(now, from, data);
            return;
        };
        let reached = self.sockets.reached(from).unwrap_or(from);
        let status = || Status {
            name: &self.motd.name,
            map: &self.motd.sub_name,
            players: self.sessions.player_names(),
            max_players: self.max_players.get(),
            port: reached.port(),
            ip: reached.ip(),
        };
        if let Some(reply) = query.answer(data, from, now, status) {
            self.replies.push((from, reply));
        }
    }

    async fn flush(&mut self, now: Instant) {
        for (to, reply) in std::mem::take(&mut self.replies) {
            let _ = self.sockets.send_to(&reply, to).await;
        }
        while let Some((to, datagram)) = self.links.poll_transmit(now) {
            let _ = self.sockets.send_to(&datagram, to).await;
        }
    }
}

/// The next signaling request; never, without NetherNet or once its listener has ended.
async fn next_request(signaling: &mut Option<mpsc::UnboundedReceiver<Request>>) -> Request {
    match signaling {
        Some(requests) => match requests.recv().await {
            Some(request) => request,
            None => {
                *signaling = None;
                pending().await
            }
        },
        None => pending().await,
    }
}

async fn lan_recv(lan: Option<&Lan>, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
    match lan {
        Some(lan) => lan.socket.recv_from(buf).await,
        None => pending().await,
    }
}
