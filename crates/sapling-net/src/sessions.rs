//! Every connection of one listener: logins in progress, then players. Network-free: it reacts to
//! the transports' events and the game's outbound commands, and queues replies on the links.

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use acacia_auth::login::verify::{VerifiedLogin, Verifier};
use acacia_nethernet::host::{Answer, PeerId};
use acacia_proto::RawPacket;
use acacia_proto::packets::{Disconnect, DisconnectContent};
use acacia_proto::types::DisconnectFailReason;
use acacia_session::server::ServerConnection;
use acacia_session::server::login::{LoginConfig, ServerLogin};
use bytes::Bytes;
use p384::ecdsa::{SigningKey, VerifyingKey};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::config::NetConfig;
use crate::event::{ConnId, NetEvent, Outbound};
use crate::links::{Link, LinkEvent, Links, Refusal};

struct Pending {
    login: ServerLogin,
    deadline: Instant,
    /// The key a NetherNet offer was signed with; the Login must come from the same one.
    offer_key: Option<VerifyingKey>,
}

struct Player {
    id: ConnId,
    conn: ServerConnection,
    uuid: Uuid,
}

pub(crate) struct Sessions {
    cfg: NetConfig,
    verifier: Verifier,
    inbox: mpsc::UnboundedSender<NetEvent>,
    /// Keys of answered NetherNet offers whose connection has not opened yet.
    offers: HashMap<PeerId, VerifyingKey>,
    logins: HashMap<Link, Pending>,
    players: HashMap<Link, Player>,
    links: HashMap<ConnId, Link>,
    /// Identities in play. Reserved before `Joined` is sent, so two logins of one account cannot
    /// both reach the game (dragonfly #1364).
    identities: HashSet<Uuid>,
    next_id: u64,
}

impl Sessions {
    pub fn new(cfg: NetConfig, verifier: Verifier, inbox: mpsc::UnboundedSender<NetEvent>) -> Self {
        Self {
            cfg,
            verifier,
            inbox,
            offers: HashMap::new(),
            logins: HashMap::new(),
            players: HashMap::new(),
            links: HashMap::new(),
            identities: HashSet::new(),
            next_id: 1,
        }
    }

    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    pub fn online_mode(&self) -> bool {
        self.cfg.online_mode
    }

    /// False once the game dropped its inbox: nobody is left to play for.
    pub fn has_game(&self) -> bool {
        !self.inbox.is_closed()
    }

    /// Answers a NetherNet offer that reached us at `local`, if its identity is good enough for
    /// this server: online mode wants the auth service's token, offline takes any or none.
    pub fn admit_offer(&mut self, links: &mut Links, offer: &str, local: SocketAddr, trickle: bool, now: Instant) -> Result<Answer, Refusal> {
        let host = links.nethernet.as_mut().ok_or(Refusal::Full)?;
        let identity = acacia_nethernet::verify_offer(offer).map_err(|error| {
            tracing::debug!(%error, "offer refused");
            Refusal::Identity
        })?;
        if self.cfg.online_mode && identity.as_ref().is_none_or(|identity| identity.self_signed) {
            return Err(Refusal::Identity);
        }
        let answered = if trickle { host.trickle_answer(offer, local, now) } else { host.answer(offer, local, now) };
        let answer = answered.map_err(|error| match error {
            acacia_nethernet::Error::HostFull => Refusal::Full,
            error => {
                tracing::debug!(%error, "offer refused");
                Refusal::Offer
            }
        })?;
        if let Some(identity) = identity {
            self.offers.insert(answer.peer, identity.key);
        }
        Ok(answer)
    }

    pub fn on_event(&mut self, links: &mut Links, event: LinkEvent, now: Instant) {
        match event {
            LinkEvent::Connected(link) => {
                let key = SigningKey::random(&mut rand_core::OsRng);
                let (conn, offer_key) = match link {
                    Link::RakNet(_) => (ServerConnection::new(key), None),
                    Link::NetherNet(peer) => (ServerConnection::nethernet(key), self.offers.remove(&peer)),
                };
                let cfg = LoginConfig { require_authenticated: self.cfg.online_mode, ..LoginConfig::default() };
                self.logins.insert(link, Pending { login: ServerLogin::new(conn, cfg), deadline: now + self.cfg.login_timeout, offer_key });
            }
            LinkEvent::Message(link, msg) if self.players.contains_key(&link) => self.on_game_message(links, link, &msg, now),
            LinkEvent::Message(link, msg) => self.on_login_message(links, link, &msg, now),
            LinkEvent::Disconnected(link, reason) => {
                if let Link::NetherNet(peer) = link {
                    self.offers.remove(&peer);
                }
                self.logins.remove(&link);
                self.forget_player(link, reason);
            }
        }
    }

    pub fn on_outbound(&mut self, links: &mut Links, command: Outbound, now: Instant) {
        match command {
            Outbound::Send { conn, packets } => {
                let Some((&link, player)) = self.links.get(&conn).and_then(|link| Some((link, self.players.get_mut(link)?))) else { return };
                links.send(link, player.conn.encode(&packets), now);
                if links.queued_bytes(link) > self.cfg.max_queued_bytes {
                    self.close_player(links, link, "send backlog over the limit", "disconnectionScreen.timeout", now);
                }
            }
            Outbound::Kick { conn, message } => {
                if let Some(&link) = self.links.get(&conn) {
                    self.close_player(links, link, "kicked", &message, now);
                }
            }
        }
    }

    /// When the next login runs out of time.
    pub fn next_deadline(&self) -> Option<Instant> {
        self.logins.values().map(|p| p.deadline).min()
    }

    pub fn expire_logins(&mut self, links: &mut Links, now: Instant) {
        let late: Vec<Link> = self.logins.iter().filter(|(_, p)| p.deadline <= now).map(|(&link, _)| link).collect();
        for link in late {
            tracing::debug!(%link, "login timed out");
            self.logins.remove(&link);
            links.close(link, now);
        }
    }

    /// Tells every player the server is closing and drops all connections.
    pub fn close_all(&mut self, links: &mut Links, now: Instant) {
        for link in self.logins.drain().map(|(link, _)| link).collect::<Vec<_>>() {
            links.close(link, now);
        }
        for link in self.players.keys().copied().collect::<Vec<_>>() {
            self.close_player(links, link, "server closed", "disconnect.closed", now);
        }
    }

    fn on_login_message(&mut self, links: &mut Links, link: Link, msg: &[u8], now: Instant) {
        let Some(pending) = self.logins.get_mut(&link) else { return };
        match pending.login.handle(msg, &self.verifier, unix_now()) {
            Ok(step) => {
                for message in step.send {
                    links.send(link, message, now);
                }
                if let Some(login) = step.done {
                    self.admit(links, link, login, now);
                }
            }
            Err(rejected) => {
                tracing::debug!(%link, error = %rejected.error, "login rejected");
                self.logins.remove(&link);
                if let Some(notice) = rejected.send {
                    links.send(link, notice, now);
                }
                links.close(link, now);
            }
        }
    }

    fn admit(&mut self, links: &mut Links, link: Link, login: Box<VerifiedLogin>, now: Instant) {
        let Some(pending) = self.logins.remove(&link) else { return };
        let mut conn = pending.login.into_connection();
        let uuid = login.identity.uuid;
        // Someone else's Login over a connection whose offer another key signed.
        let refusal = if pending.offer_key.is_some_and(|key| key != VerifyingKey::from(&login.client_key)) {
            Some((DisconnectFailReason::NotAuthenticated, "disconnectionScreen.notAuthenticated"))
        } else if self.players.len() >= self.cfg.max_players {
            Some((DisconnectFailReason::ServerFull, "disconnectionScreen.serverFull"))
        } else if !self.identities.insert(uuid) {
            Some((DisconnectFailReason::LoggedInOtherLocation, "disconnectionScreen.loggedinOtherLocation"))
        } else {
            None
        };
        if let Some((reason, message)) = refusal {
            tracing::debug!(%link, name = %login.identity.display_name, ?reason, "login refused");
            links.send(link, conn.encode(&[disconnect(reason, message)]), now);
            links.close(link, now);
            return;
        }
        let id = ConnId(self.next_id);
        self.next_id += 1;
        self.links.insert(id, link);
        self.players.insert(link, Player { id, conn, uuid });
        let _ = self.inbox.send(NetEvent::Joined { conn: id, addr: links.remote_addr(link), login });
    }

    fn on_game_message(&mut self, links: &mut Links, link: Link, msg: &[u8], now: Instant) {
        let Some(player) = self.players.get_mut(&link) else { return };
        let packets = player.conn.decode(msg).map_err(|e| e.to_string()).and_then(|packets| {
            packets.into_iter().map(|p| RawPacket::parse(p).map_err(|e| e.to_string())).collect::<Result<Vec<_>, _>>()
        });
        match packets {
            Ok(packets) => {
                for packet in packets {
                    let _ = self.inbox.send(NetEvent::Packet { conn: player.id, packet });
                }
            }
            Err(error) => {
                tracing::debug!(%link, %error, "unreadable batch");
                self.close_player(links, link, "unreadable batch", "disconnectionScreen.badPacket", now);
            }
        }
    }

    /// Sends the disconnect screen, closes the connection and reports the player as gone.
    fn close_player(&mut self, links: &mut Links, link: Link, reason: &str, message: &str, now: Instant) {
        if let Some(player) = self.players.get_mut(&link) {
            let notice = player.conn.encode(&[disconnect(DisconnectFailReason::Kicked, message)]);
            links.send(link, notice, now);
        }
        links.close(link, now);
        self.forget_player(link, reason.to_owned());
    }

    fn forget_player(&mut self, link: Link, reason: String) {
        let Some(player) = self.players.remove(&link) else { return };
        self.links.remove(&player.id);
        self.identities.remove(&player.uuid);
        let _ = self.inbox.send(NetEvent::Left { conn: player.id, reason });
    }
}

fn disconnect(reason: DisconnectFailReason, message: &str) -> Bytes {
    let content = DisconnectContent { message: message.into(), filtered_message: String::new() };
    crate::encode(&Disconnect { reason, hide_disconnect_reason: false, content: Some(content) })
}

fn unix_now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}
