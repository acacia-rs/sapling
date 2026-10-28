//! The GameSpy4 query (`enable-query`), answered on the game ports as PocketMine and Java's query
//! answer it. BDS has no such key and answered neither port (research/captures/bds-query.txt), so
//! this is the standard protocol: a handshake (type 9) is answered with a challenge token, and a
//! stat request (type 0) bearing it with the basic status, or, padded to eight bytes, the full one
//! with every player's name.

use std::hash::{BuildHasher, Hasher, RandomState};
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

const MAGIC: [u8; 2] = [0xFE, 0xFD];
const HANDSHAKE: u8 = 9;
const STAT: u8 = 0;
/// A token is good for one to two of these, as Java's.
const TOKEN_LIFE: Duration = Duration::from_secs(30);

/// What a status answer tells.
pub(crate) struct Status<'a> {
    pub name: &'a str,
    pub map: &'a str,
    pub players: Vec<&'a str>,
    pub max_players: usize,
    pub port: u16,
    pub ip: IpAddr,
}

pub(crate) struct Query {
    /// The token secrets now and before the last turn.
    secrets: [RandomState; 2],
    turned: Instant,
}

impl Query {
    pub fn new(now: Instant) -> Self {
        Self { secrets: [RandomState::new(), RandomState::new()], turned: now }
    }

    /// Whether `data` is a query rather than RakNet or NetherNet.
    pub fn is_query(data: &[u8]) -> bool {
        data.len() >= 7 && data[..2] == MAGIC
    }

    fn token(secret: &RandomState, from: IpAddr) -> i32 {
        let mut hasher = secret.build_hasher();
        hasher.write(&match from {
            IpAddr::V4(ip) => ip.to_ipv6_mapped().octets(),
            IpAddr::V6(ip) => ip.octets(),
        });
        (hasher.finish() & 0x7FFF_FFFF) as i32
    }

    /// The answer to the query `data` from `from`; none for a malformed one or a stale token.
    pub fn answer<'a>(&mut self, data: &[u8], from: SocketAddr, now: Instant, status: impl FnOnce() -> Status<'a>) -> Option<Vec<u8>> {
        let idle = now.duration_since(self.turned);
        if idle >= TOKEN_LIFE {
            let before = if idle >= TOKEN_LIFE * 2 { RandomState::new() } else { self.secrets[0].clone() };
            self.secrets = [RandomState::new(), before];
            self.turned = now;
        }
        let (kind, session, rest) = (data[2], &data[3..7], &data[7..]);
        let mut out = vec![kind];
        out.extend_from_slice(session);
        match kind {
            HANDSHAKE => {
                out.extend_from_slice(Self::token(&self.secrets[0], from.ip()).to_string().as_bytes());
                out.push(0);
            }
            STAT if rest.len() >= 4 => {
                let token = i32::from_be_bytes(rest[..4].try_into().ok()?);
                if !self.secrets.iter().any(|secret| Self::token(secret, from.ip()) == token) {
                    return None;
                }
                let status = status();
                if rest.len() >= 8 { full(&mut out, &status) } else { basic(&mut out, &status) }
            }
            _ => return None,
        }
        Some(out)
    }
}

fn text(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(value.as_bytes());
    out.push(0);
}

fn basic(out: &mut Vec<u8>, status: &Status) {
    for value in [status.name, "SMP", status.map, &status.players.len().to_string(), &status.max_players.to_string()] {
        text(out, value);
    }
    out.extend_from_slice(&status.port.to_le_bytes());
    text(out, &status.ip.to_string());
}

fn full(out: &mut Vec<u8>, status: &Status) {
    out.extend_from_slice(b"splitnum\0\x80\0");
    let pairs = [
        ("hostname", status.name.to_owned()),
        ("gametype", "SMP".to_owned()),
        ("game_id", "MINECRAFTPE".to_owned()),
        ("version", acacia_proto::GAME_VERSION.to_owned()),
        ("plugins", String::new()),
        ("map", status.map.to_owned()),
        ("numplayers", status.players.len().to_string()),
        ("maxplayers", status.max_players.to_string()),
        ("hostport", status.port.to_string()),
        ("hostip", status.ip.to_string()),
    ];
    for (key, value) in &pairs {
        text(out, key);
        text(out, value);
    }
    out.push(0);
    out.extend_from_slice(b"\x01player_\0\0");
    status.players.iter().for_each(|name| text(out, name));
    out.push(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status() -> Status<'static> {
        Status { name: "hub", map: "Spawn", players: vec!["Alex", "Steve"], max_players: 20, port: 19132, ip: [127, 0, 0, 1].into() }
    }

    #[test]
    fn a_handshake_token_opens_the_basic_and_full_status() {
        let (mut query, now, from) = (Query::new(Instant::now()), Instant::now(), SocketAddr::from(([10, 0, 0, 2], 5000)));
        let hand = query.answer(&[0xFE, 0xFD, 9, 0, 0, 0, 7], from, now, status).unwrap();
        assert_eq!(hand[..5], [9, 0, 0, 0, 7]);
        let token: i32 = std::str::from_utf8(&hand[5..hand.len() - 1]).unwrap().parse().unwrap();
        let mut ask = vec![0xFE, 0xFD, 0, 0, 0, 0, 7];
        ask.extend_from_slice(&token.to_be_bytes());
        let basic = query.answer(&ask, from, now, status).unwrap();
        assert_eq!(basic, b"\0\0\0\0\x07hub\0SMP\0Spawn\x002\x0020\0\xbc\x4a127.0.0.1\0");
        ask.extend_from_slice(&[0; 4]);
        let full = query.answer(&ask, from, now, status).unwrap();
        let tail = b"maxplayers\x0020\0hostport\x0019132\0hostip\x00127.0.0.1\0\0\x01player_\0\0Alex\0Steve\0\0";
        assert!(full.starts_with(b"\0\0\0\0\x07splitnum\0\x80\0hostname\0hub\0gametype\0SMP\0game_id\0MINECRAFTPE\0") && full.ends_with(tail));
        assert!(query.answer(&ask, SocketAddr::from(([10, 0, 0, 3], 5000)), now, status).is_none(), "another address's token");
        assert!(query.answer(&ask, from, now + TOKEN_LIFE * 2, status).is_none(), "a stale token");
    }
}
