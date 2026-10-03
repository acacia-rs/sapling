//! The socket loop: datagrams in, the RakNet server's output out, timers in between.

use std::time::{Duration, Instant};

use acacia_raknet::Server;
use bytes::Bytes;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use crate::config::Motd;
use crate::event::Outbound;
use crate::sessions::Sessions;

/// Longer than any datagram a peer may send (the RakNet MTU tops out at 1492).
const DATAGRAM_BUF: usize = 2048;
/// Work taken per wake-up before sending replies, so a flood on one side cannot starve the other.
const BURST: usize = 256;
/// Wake-up when nothing is scheduled, to notice a game that went away.
const IDLE_WAKE: Duration = Duration::from_millis(500);

pub(crate) struct Driver {
    pub socket: UdpSocket,
    pub server: Server,
    pub sessions: Sessions,
    pub outbound: mpsc::UnboundedReceiver<Outbound>,
    pub motd: Motd,
    pub guid: u64,
    pub max_players: usize,
}

impl Driver {
    /// Runs until the game drops its outbox or inbox, then says goodbye to every player.
    pub async fn run(mut self) {
        let port = self.socket.local_addr().map_or(0, |a| a.port());
        let mut advertised = usize::MAX;
        let mut buf = vec![0u8; DATAGRAM_BUF];
        while self.sessions.has_game() {
            // Events first: the replies they queue must leave in this turn, not at the next wake-up.
            let now = Instant::now();
            while let Some(event) = self.server.poll_event() {
                self.sessions.on_event(&mut self.server, event, now);
            }
            self.sessions.expire_logins(&mut self.server, now);
            if advertised != self.sessions.player_count() {
                advertised = self.sessions.player_count();
                self.server.set_motd(self.motd.pong(self.guid, advertised, self.max_players, port));
            }
            self.flush(now).await;
            let timers = self.server.poll_timeout().into_iter().chain(self.sessions.next_deadline());
            let deadline = timers.min().unwrap_or(now + IDLE_WAKE);
            tokio::select! {
                received = self.socket.recv_from(&mut buf) => {
                    let now = Instant::now();
                    let mut received = received.ok();
                    for _ in 0..BURST {
                        let Some((len, from)) = received else { break };
                        self.server.handle_datagram(now, from, Bytes::copy_from_slice(&buf[..len]));
                        received = self.socket.try_recv_from(&mut buf).ok();
                    }
                }
                command = self.outbound.recv() => {
                    let now = Instant::now();
                    let Some(command) = command else { break };
                    self.sessions.on_outbound(&mut self.server, command, now);
                    for _ in 0..BURST {
                        let Ok(command) = self.outbound.try_recv() else { break };
                        self.sessions.on_outbound(&mut self.server, command, now);
                    }
                }
                _ = tokio::time::sleep_until(deadline.into()) => self.server.handle_timeout(Instant::now()),
            }
        }
        let now = Instant::now();
        self.sessions.close_all(&mut self.server, now);
        self.flush(now).await;
    }

    async fn flush(&mut self, now: Instant) {
        while let Some((to, datagram)) = self.server.poll_transmit(now) {
            let _ = self.socket.send_to(&datagram, to).await;
        }
    }
}
