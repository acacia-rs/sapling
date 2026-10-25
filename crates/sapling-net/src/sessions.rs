//! Every connection of one listener: logins in progress, then players. Network-free: it reacts to
//! the transports' events and the game's outbound commands, and queues replies on the links.

mod offers;

use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::time::Instant;

use acacia_auth::login::verify::{VerifiedLogin, Verifier};
use acacia_nethernet::host::PeerId;
use acacia_proto::RawPacket;
use acacia_proto::types::DisconnectFailReason;
use acacia_session::server::ServerConnection;
use acacia_session::server::login::{LoginConfig, ServerLogin};
use p384::ecdsa::{SigningKey, VerifyingKey};
use uuid::Uuid;

use crate::access;
use crate::admission::Throttle;
use crate::config::NetConfig;
use crate::congestion::Congestion;
use crate::event::{ConnId, NetEvent, Outbound};
use crate::limits::{Meter, Verdict};
use crate::links::{Link, LinkEvent, Links};
use crate::players::{Player, disconnect};
use crate::router::Router;

/// The disconnect screen of a player whose server, or whose part of it, stopped.
const CLOSED: &str = "disconnect.closed";
/// Java's screen for a flood, which the Bedrock client's strings also hold.
const SPAM: &str = "disconnect.spam";

struct Pending {
    login: ServerLogin,
    deadline: Instant,
    /// The key a NetherNet offer was signed with; the Login must come from the same one.
    offer_key: Option<VerifyingKey>,
    ip: IpAddr,
}

pub(crate) struct Sessions {
    cfg: NetConfig,
    verifier: Verifier,
    router: Router,
    congestion: Congestion,
    /// Keys of answered NetherNet offers whose connection has not opened yet.
    offers: HashMap<PeerId, VerifyingKey>,
    logins: HashMap<Link, Pending>,
    players: HashMap<Link, Player>,
    links: HashMap<ConnId, Link>,
    /// Identities in play. Reserved before `Joined` is sent, so two logins of one account cannot
    /// both reach the game (dragonfly #1364).
    identities: HashSet<Uuid>,
    next_id: u64,
    throttle: Throttle,
    /// The access lists' ban count the players were last checked against.
    bans_seen: u64,
}

impl Sessions {
    pub fn new(cfg: NetConfig, verifier: Verifier, router: Router, congestion: Congestion) -> Self {
        let (offers, logins, players, links) = (HashMap::new(), HashMap::new(), HashMap::new(), HashMap::new());
        let (throttle, bans_seen) = (Throttle::new(cfg.admission.clone()), cfg.access.as_ref().map_or(0, |access| access.bans_changed()));
        Self { cfg, verifier, router, congestion, offers, logins, players, links, identities: HashSet::new(), next_id: 1, throttle, bans_seen }
    }

    /// Re-reads every player's unsent backlog: the transports send and get acked between the
    /// game's sends.
    pub fn track_backlogs(&mut self, links: &Links) {
        for (&link, player) in &mut self.players {
            player.track_backlog(links.queued_bytes(link), &self.cfg, &self.congestion);
        }
    }

    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    pub fn online_mode(&self) -> bool {
        self.cfg.online_mode
    }

    /// False once the game dropped its inboxes: nobody is left to play for.
    pub fn has_game(&self) -> bool {
        self.router.is_open()
    }

    pub fn on_event(&mut self, links: &mut Links, event: LinkEvent, now: Instant) {
        match event {
            LinkEvent::Connected(link) => {
                let ip = links.remote_addr(link).ip();
                let open = self.logins.values().filter(|p| p.ip == ip).count() + self.players.values().filter(|p| p.ip == ip).count();
                if let Err(turned) = self.throttle.admit(ip, open, now) {
                    tracing::debug!(%link, ?turned, "connection turned away");
                    links.close(link, now);
                    return;
                }
                let key = SigningKey::random(&mut rand_core::OsRng);
                let (conn, offer_key) = match link {
                    Link::RakNet(_) => (ServerConnection::new(key), None),
                    Link::NetherNet(peer) => (ServerConnection::nethernet(key), self.offers.remove(&peer)),
                };
                let (compression, compression_threshold) = (self.cfg.compression.settings(), self.cfg.compression_threshold);
                let cfg = LoginConfig { require_authenticated: self.cfg.online_mode, compression, compression_threshold, ..LoginConfig::default() };
                self.logins.insert(link, Pending { login: ServerLogin::new(conn, cfg), deadline: now + self.cfg.login_timeout, offer_key, ip });
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
                let queued = links.queued_bytes(link);
                if queued > self.cfg.max_queued_bytes {
                    self.close_player(links, link, "send backlog over the limit", "disconnectionScreen.timeout", now);
                } else {
                    player.track_backlog(queued, &self.cfg, &self.congestion);
                }
            }
            Outbound::Kick { conn, reason, message } => {
                if let Some(&link) = self.links.get(&conn) {
                    self.close_player_for(links, link, "kicked", reason, &message, now);
                }
            }
            Outbound::Route { conn, from, to } => {
                if let (false, Some(&link)) = (self.router.reroute(conn, from, to), self.links.get(&conn)) {
                    self.close_player(links, link, "its shard is gone", CLOSED, now);
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

    /// Drops the players a ban made since the last call names, with the ban's screen.
    pub fn enforce_bans(&mut self, links: &mut Links, now: Instant) {
        let Some(access) = &self.cfg.access else { return };
        if access.bans_changed() == self.bans_seen {
            return;
        }
        self.bans_seen = access.bans_changed();
        let banned: Vec<(Link, String)> = self.players.iter().filter_map(|(&link, p)| Some((link, access.ban_screen(&p.name, &p.xuid, p.ip)?))).collect();
        for (link, screen) in banned {
            self.close_player(links, link, "banned", &screen, now);
        }
    }

    /// Tells every player the server is closing and drops all connections.
    pub fn close_all(&mut self, links: &mut Links, now: Instant) {
        for link in self.logins.drain().map(|(link, _)| link).collect::<Vec<_>>() {
            links.close(link, now);
        }
        for link in self.players.keys().copied().collect::<Vec<_>>() {
            self.close_player(links, link, "server closed", CLOSED, now);
        }
    }

    fn on_login_message(&mut self, links: &mut Links, link: Link, msg: &[u8], now: Instant) {
        let Some(pending) = self.logins.get_mut(&link) else { return };
        match pending.login.handle(msg, &self.verifier, access::unix_now() as i64) {
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
        let identity = &login.identity;
        let (listed, ignores_limit) = self.cfg.access.as_ref().map_or((None, false), |access| access.admit(&identity.display_name, &identity.xuid, pending.ip));
        // Someone else's Login over a connection whose offer another key signed.
        let refusal = if pending.offer_key.is_some_and(|key| key != VerifyingKey::from(&login.client_key)) {
            Some((DisconnectFailReason::NotAuthenticated, "disconnectionScreen.notAuthenticated".to_owned()))
        } else if let Some(listed) = listed {
            Some(listed.screen())
        } else if self.players.len() >= self.cfg.max_players.get() && !ignores_limit {
            Some((DisconnectFailReason::ServerFull, "disconnectionScreen.serverFull".to_owned()))
        } else if !self.identities.insert(identity.uuid) {
            Some((DisconnectFailReason::LoggedInOtherLocation, "disconnectionScreen.loggedinOtherLocation".to_owned()))
        } else {
            None
        };
        if let Some((reason, message)) = refusal {
            tracing::debug!(%link, name = %identity.display_name, ?reason, "login refused");
            links.send(link, conn.encode(&[disconnect(reason, &message)]), now);
            links.close(link, now);
            return;
        }
        if let Some(access) = &self.cfg.access {
            access.joined(&identity.display_name, pending.ip);
        }
        let id = ConnId(self.next_id);
        self.next_id += 1;
        self.links.insert(id, link);
        let (uuid, name, xuid) = (identity.uuid, identity.display_name.clone(), identity.xuid.clone());
        let meter = (!pending.ip.is_loopback()).then(|| Meter::new(now));
        self.players.insert(link, Player { id, conn, uuid, name, xuid, ip: pending.ip, congested: false, meter });
        if !self.router.deliver(NetEvent::Joined { conn: id, addr: links.remote_addr(link), login }) {
            self.close_player(links, link, "its shard is gone", CLOSED, now);
        }
    }

    fn on_game_message(&mut self, links: &mut Links, link: Link, msg: &[u8], now: Instant) {
        let Some(player) = self.players.get_mut(&link) else { return };
        let packets = player.conn.decode(msg).map_err(|e| e.to_string()).and_then(|packets| {
            packets.into_iter().map(|p| RawPacket::parse(p).map_err(|e| e.to_string())).collect::<Result<Vec<_>, _>>()
        });
        let mut packets = match packets {
            Ok(packets) => packets,
            Err(error) => {
                tracing::debug!(%link, %error, "unreadable batch");
                return self.close_player(links, link, "unreadable batch", "disconnectionScreen.badPacket", now);
            }
        };
        let (id, limits, mut flood) = (player.id, &self.cfg.packet_limits, false);
        if let Some(meter) = &mut player.meter {
            packets.retain(|packet| match meter.count(limits, packet.id, now) {
                Verdict::Pass => true,
                Verdict::Drop => false,
                Verdict::Close => {
                    flood = true;
                    false
                }
            });
        }
        if flood {
            return self.close_player(links, link, "packet flood", SPAM, now);
        }
        if !packets.into_iter().all(|packet| self.router.deliver(NetEvent::Packet { conn: id, packet })) {
            self.close_player(links, link, "its shard is gone", CLOSED, now);
        }
    }

    /// Sends the disconnect screen, closes the connection and reports the player as gone.
    fn close_player(&mut self, links: &mut Links, link: Link, reason: &str, message: &str, now: Instant) {
        self.close_player_for(links, link, reason, DisconnectFailReason::Kicked, message, now);
    }

    fn close_player_for(&mut self, links: &mut Links, link: Link, reason: &str, code: DisconnectFailReason, message: &str, now: Instant) {
        if let Some(player) = self.players.get_mut(&link) {
            let notice = player.conn.encode(&[disconnect(code, message)]);
            links.send(link, notice, now);
        }
        links.close(link, now);
        self.forget_player(link, reason.to_owned());
    }

    fn forget_player(&mut self, link: Link, reason: String) {
        let Some(player) = self.players.remove(&link) else { return };
        self.congestion.set(player.id, false);
        self.links.remove(&player.id);
        self.identities.remove(&player.uuid);
        if let Some(access) = &self.cfg.access {
            access.left(&player.name);
        }
        self.router.deliver(NetEvent::Left { conn: player.id, reason });
    }
}