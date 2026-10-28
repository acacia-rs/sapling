use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use acacia_nethernet::lan::{DISCOVERY_PORT, ServerData};
use acacia_proto::packets::NetworkSettingsCompressionAlgorithm;
use acacia_proto::{GAME_VERSION, PROTOCOL_VERSION};
use p384::ecdsa::SigningKey;

use crate::access::AccessLists;
use crate::admission::Admission;
use crate::limits::PacketLimits;

#[derive(Debug, Clone)]
pub struct NetConfig {
    /// Admit only logins the authorization service or Mojang signed.
    pub online_mode: bool,
    /// From the connection to PlayStatus(LoginSuccess); slower clients are dropped.
    pub login_timeout: Duration,
    pub max_players: crate::PlayerLimit,
    pub motd: Motd,
    /// A player whose unsent backlog grows past this is kicked rather than buffered without end.
    pub max_queued_bytes: usize,
    /// A connection with more than this unsent is congested ([`crate::Outbox::is_congested`]) until
    /// it drains below `drained_bytes`.
    pub congested_bytes: usize,
    pub drained_bytes: usize,
    /// NetherNet beside RakNet. Off by default: a vanilla client that finds it stops using RakNet.
    pub nethernet: Option<NetherNet>,
    /// Limits on each address's connections.
    pub admission: Admission,
    /// Bans and the allowlist each login is checked against; none by default.
    pub access: Option<AccessLists>,
    /// How batches are compressed: BDS's `compression-algorithm` (default zlib).
    pub compression: Compression,
    /// Batches at least this long are compressed: `compression-threshold` (default 1, as BDS's).
    pub compression_threshold: u16,
    /// What one player's client may send a second.
    pub packet_limits: PacketLimits,
    /// An IPv6 socket beside the game's (BDS's `server-portv6`): RakNet and the query on it too,
    /// and its port in the pong. None by default.
    pub bind_v6: Option<SocketAddr>,
    /// Answer the GameSpy4 query on the game ports (`enable-query`; query.rs). Off by default.
    pub query: bool,
}

impl Default for NetConfig {
    fn default() -> Self {
        Self {
            online_mode: true,
            login_timeout: Duration::from_secs(10),
            max_players: crate::PlayerLimit::default(),
            motd: Motd::default(),
            max_queued_bytes: 16 * 1024 * 1024,
            congested_bytes: 1024 * 1024,
            drained_bytes: 256 * 1024,
            nethernet: None,
            admission: Admission::default(),
            access: None,
            compression: Compression::default(),
            compression_threshold: 1,
            packet_limits: PacketLimits::default(),
            bind_v6: None,
            query: false,
        }
    }
}

/// A batch compression algorithm, under the names of `compression-algorithm`'s values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Compression {
    /// Raw deflate, which BDS calls zlib.
    #[default]
    Zlib,
    Snappy,
}

impl Compression {
    pub(crate) fn settings(self) -> NetworkSettingsCompressionAlgorithm {
        match self {
            Self::Zlib => NetworkSettingsCompressionAlgorithm::Deflate,
            Self::Snappy => NetworkSettingsCompressionAlgorithm::Snappy,
        }
    }
}

/// NetherNet (WebRTC) hosting; see docs/DESIGN.md "NetherNet". Direct connect is signaled over
/// HTTP on the TCP side of the game port, and its datagrams share the game's UDP socket.
#[derive(Debug, Clone)]
pub struct NetherNet {
    /// Signs every answer. Clients pin it per server address on first use, so keep it across
    /// restarts ([`crate::host_key`]).
    pub host_key: SigningKey,
    /// The address players reach the server at, when it is not the one their signaling arrives on
    /// (a forwarded port): answers advertise it as the WebRTC candidate.
    pub public_ip: Option<IpAddr>,
    /// Answer LAN discovery, which lists the server for clients on the same network.
    pub lan: Option<Lan>,
}

impl NetherNet {
    pub fn new(host_key: SigningKey) -> Self {
        Self { host_key, public_ip: None, lan: None }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lan {
    /// Vanilla clients only broadcast to 7551; 0 picks a free port (tests).
    pub port: u16,
}

impl Default for Lan {
    fn default() -> Self {
        Self { port: DISCOVERY_PORT }
    }
}

/// What the server list shows, sent in answer to unconnected pings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Motd {
    pub name: String,
    pub sub_name: String,
    pub game_mode: String,
}

impl Default for Motd {
    fn default() -> Self {
        Self { name: "sapling".into(), sub_name: "sapling".into(), game_mode: "Survival".into() }
    }
}

impl Motd {
    /// The pong payload, with the IPv4 and IPv6 ports; a `;` in a name would shift every field
    /// after it.
    pub(crate) fn pong(&self, guid: u64, players: usize, max_players: usize, [port, port_v6]: [u16; 2]) -> String {
        let clean = |s: &str| s.replace(';', "");
        let (name, sub_name, game_mode) = (clean(&self.name), clean(&self.sub_name), clean(&self.game_mode));
        format!("MCPE;{name};{PROTOCOL_VERSION};{GAME_VERSION};{players};{max_players};{guid};{sub_name};{game_mode};1;{port};{port_v6};0;")
    }

    /// The body of `GET /v1/join`: NetherNet's pong.
    pub(crate) fn status(&self, players: usize, max_players: usize) -> String {
        serde_json::json!({
            "name": self.name,
            "protocol": PROTOCOL_VERSION,
            "version": GAME_VERSION,
            "level": self.sub_name,
            "players": players,
            "maxPlayers": max_players,
            "gameType": self.game_type(),
        })
        .to_string()
    }

    /// What a LAN discovery answer carries.
    pub(crate) fn server_data(&self, players: usize, max_players: usize, online_mode: bool) -> ServerData {
        ServerData {
            server_name: self.name.clone(),
            protocol: PROTOCOL_VERSION,
            version: GAME_VERSION.to_owned(),
            level_name: self.sub_name.clone(),
            player_count: players as i32,
            max_player_count: max_players as i32,
            game_type: self.game_type(),
            accepts_online_auth: online_mode,
            accepts_self_signed_auth: !online_mode,
            connection_type: 4,
            ..ServerData::default()
        }
    }

    fn game_type(&self) -> i32 {
        match self.game_mode.to_ascii_lowercase().as_str() {
            "creative" => 1,
            "adventure" => 2,
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pong_has_the_fields_the_server_list_reads() {
        let motd = Motd { name: "a;b".into(), ..Motd::default() };
        let pong = motd.pong(7, 3, 20, [19132, 19133]);
        let fields: Vec<&str> = pong.split(';').collect();
        assert_eq!(fields[..2], ["MCPE", "ab"]);
        assert_eq!(fields[2], PROTOCOL_VERSION.to_string());
        assert_eq!(fields[4..12], ["3", "20", "7", "sapling", "Survival", "1", "19132", "19133"]);
    }

    #[test]
    fn nethernet_status_and_lan_advert_carry_the_same_counts() {
        let motd = Motd { name: "hub \"1\"".into(), game_mode: "Creative".into(), ..Motd::default() };
        let status: serde_json::Value = serde_json::from_str(&motd.status(3, 20)).unwrap();
        assert_eq!((status["name"].as_str(), status["players"].as_i64(), status["gameType"].as_i64()), (Some("hub \"1\""), Some(3), Some(1)));
        let data = motd.server_data(3, 20, true);
        assert_eq!((data.player_count, data.max_player_count, data.game_type, data.accepts_self_signed_auth), (3, 20, 1, false));
    }
}
