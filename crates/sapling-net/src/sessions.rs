//! Every connection of one listener: logins in progress, then players. Network-free: it reacts to
//! the RakNet server's events and the game's outbound commands, and queues replies on the server.

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use acacia_auth::login::verify::{VerifiedLogin, Verifier};
use acacia_proto::RawPacket;
use acacia_proto::packets::{Disconnect, DisconnectContent};
use acacia_proto::types::DisconnectFailReason;
use acacia_raknet::{Reliability, Server, ServerEvent};
use acacia_session::server::ServerConnection;
use acacia_session::server::login::{LoginConfig, ServerLogin};
use bytes::Bytes;
use p384::ecdsa::SigningKey;
use uuid::Uuid;

use crate::config::NetConfig;
use crate::event::{ConnId, NetEvent, Outbound};
use crate::router::Router;

/// The disconnect screen of a player whose server, or whose part of it, stopped.
const CLOSED: &str = "disconnect.closed";

struct Pending {
    login: ServerLogin,
    deadline: Instant,
}

struct Player {
    id: ConnId,
    conn: ServerConnection,
    uuid: Uuid,
}

pub(crate) struct Sessions {
    cfg: NetConfig,
    verifier: Verifier,
    router: Router,
    logins: HashMap<SocketAddr, Pending>,
    players: HashMap<SocketAddr, Player>,
    addrs: HashMap<ConnId, SocketAddr>,
    /// Identities in play. Reserved before `Joined` is sent, so two logins of one account cannot
    /// both reach the game (dragonfly #1364).
    identities: HashSet<Uuid>,
    next_id: u64,
}

impl Sessions {
    pub fn new(cfg: NetConfig, verifier: Verifier, router: Router) -> Self {
        Self { cfg, verifier, router, logins: HashMap::new(), players: HashMap::new(), addrs: HashMap::new(), identities: HashSet::new(), next_id: 1 }
    }

    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    /// False once the game dropped its inboxes: nobody is left to play for.
    pub fn has_game(&self) -> bool {
        self.router.is_open()
    }

    pub fn on_event(&mut self, server: &mut Server, event: ServerEvent, now: Instant) {
        match event {
            ServerEvent::Connected { addr, .. } => {
                let conn = ServerConnection::new(SigningKey::random(&mut rand_core::OsRng));
                let cfg = LoginConfig { require_authenticated: self.cfg.online_mode, ..LoginConfig::default() };
                self.logins.insert(addr, Pending { login: ServerLogin::new(conn, cfg), deadline: now + self.cfg.login_timeout });
            }
            ServerEvent::Message(addr, msg) if self.players.contains_key(&addr) => self.on_game_message(server, addr, &msg, now),
            ServerEvent::Message(addr, msg) => self.on_login_message(server, addr, &msg, now),
            ServerEvent::Disconnected(addr, reason) => {
                self.logins.remove(&addr);
                self.forget_player(addr, format!("{reason:?}"));
            }
        }
    }

    pub fn on_outbound(&mut self, server: &mut Server, command: Outbound, now: Instant) {
        match command {
            Outbound::Send { conn, packets } => {
                let Some((&addr, player)) = self.addrs.get(&conn).and_then(|addr| Some((addr, self.players.get_mut(addr)?))) else { return };
                server.send(addr, player.conn.encode(&packets), Reliability::ReliableOrdered);
                if server.stats(addr).is_some_and(|s| s.queued_bytes > self.cfg.max_queued_bytes) {
                    self.close_player(server, addr, "send backlog over the limit", "disconnectionScreen.timeout", now);
                }
            }
            Outbound::Kick { conn, message } => {
                if let Some(&addr) = self.addrs.get(&conn) {
                    self.close_player(server, addr, "kicked", &message, now);
                }
            }
            Outbound::Route { conn, from, to } => {
                if let (false, Some(&addr)) = (self.router.reroute(conn, from, to), self.addrs.get(&conn)) {
                    self.close_player(server, addr, "its shard is gone", CLOSED, now);
                }
            }
        }
    }

    /// When the next login runs out of time.
    pub fn next_deadline(&self) -> Option<Instant> {
        self.logins.values().map(|p| p.deadline).min()
    }

    pub fn expire_logins(&mut self, server: &mut Server, now: Instant) {
        let late: Vec<SocketAddr> = self.logins.iter().filter(|(_, p)| p.deadline <= now).map(|(&addr, _)| addr).collect();
        for addr in late {
            tracing::debug!(%addr, "login timed out");
            self.logins.remove(&addr);
            server.close(addr, now);
        }
    }

    /// Tells every player the server is closing and drops all connections.
    pub fn close_all(&mut self, server: &mut Server, now: Instant) {
        for addr in self.logins.drain().map(|(addr, _)| addr).collect::<Vec<_>>() {
            server.close(addr, now);
        }
        for addr in self.players.keys().copied().collect::<Vec<_>>() {
            self.close_player(server, addr, "server closed", CLOSED, now);
        }
    }

    fn on_login_message(&mut self, server: &mut Server, addr: SocketAddr, msg: &[u8], now: Instant) {
        let Some(pending) = self.logins.get_mut(&addr) else { return };
        match pending.login.handle(msg, &self.verifier, unix_now()) {
            Ok(step) => {
                for message in step.send {
                    server.send(addr, message, Reliability::ReliableOrdered);
                }
                if let Some(login) = step.done {
                    self.admit(server, addr, login, now);
                }
            }
            Err(rejected) => {
                tracing::debug!(%addr, error = %rejected.error, "login rejected");
                self.logins.remove(&addr);
                if let Some(notice) = rejected.send {
                    server.send(addr, notice, Reliability::ReliableOrdered);
                }
                server.close(addr, now);
            }
        }
    }

    fn admit(&mut self, server: &mut Server, addr: SocketAddr, login: Box<VerifiedLogin>, now: Instant) {
        let Some(pending) = self.logins.remove(&addr) else { return };
        let mut conn = pending.login.into_connection();
        let uuid = login.identity.uuid;
        let refusal = if self.players.len() >= self.cfg.max_players {
            Some((DisconnectFailReason::ServerFull, "disconnectionScreen.serverFull"))
        } else if !self.identities.insert(uuid) {
            Some((DisconnectFailReason::LoggedInOtherLocation, "disconnectionScreen.loggedinOtherLocation"))
        } else {
            None
        };
        if let Some((reason, message)) = refusal {
            tracing::debug!(%addr, name = %login.identity.display_name, ?reason, "login refused");
            server.send(addr, conn.encode(&[disconnect(reason, message)]), Reliability::ReliableOrdered);
            server.close(addr, now);
            return;
        }
        let id = ConnId(self.next_id);
        self.next_id += 1;
        self.addrs.insert(id, addr);
        self.players.insert(addr, Player { id, conn, uuid });
        if !self.router.deliver(NetEvent::Joined { conn: id, addr, login }) {
            self.close_player(server, addr, "its shard is gone", CLOSED, now);
        }
    }

    fn on_game_message(&mut self, server: &mut Server, addr: SocketAddr, msg: &[u8], now: Instant) {
        let Some(player) = self.players.get_mut(&addr) else { return };
        let packets = player.conn.decode(msg).map_err(|e| e.to_string()).and_then(|packets| {
            packets.into_iter().map(|p| RawPacket::parse(p).map_err(|e| e.to_string())).collect::<Result<Vec<_>, _>>()
        });
        match packets {
            Ok(packets) => {
                let id = player.id;
                let delivered = packets.into_iter().all(|packet| self.router.deliver(NetEvent::Packet { conn: id, packet }));
                if !delivered {
                    self.close_player(server, addr, "its shard is gone", CLOSED, now);
                }
            }
            Err(error) => {
                tracing::debug!(%addr, %error, "unreadable batch");
                self.close_player(server, addr, "unreadable batch", "disconnectionScreen.badPacket", now);
            }
        }
    }

    /// Sends the disconnect screen, closes the connection and reports the player as gone.
    fn close_player(&mut self, server: &mut Server, addr: SocketAddr, reason: &str, message: &str, now: Instant) {
        if let Some(player) = self.players.get_mut(&addr) {
            let notice = player.conn.encode(&[disconnect(DisconnectFailReason::Kicked, message)]);
            server.send(addr, notice, Reliability::ReliableOrdered);
        }
        server.close(addr, now);
        self.forget_player(addr, reason.to_owned());
    }

    fn forget_player(&mut self, addr: SocketAddr, reason: String) {
        let Some(player) = self.players.remove(&addr) else { return };
        self.addrs.remove(&player.id);
        self.identities.remove(&player.uuid);
        self.router.deliver(NetEvent::Left { conn: player.id, reason });
    }
}

fn disconnect(reason: DisconnectFailReason, message: &str) -> Bytes {
    let content = DisconnectContent { message: message.into(), filtered_message: String::new() };
    crate::encode(&Disconnect { reason, hide_disconnect_reason: false, content: Some(content) })
}

fn unix_now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}
