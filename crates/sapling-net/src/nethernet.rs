//! NetherNet hosting options and what the listener makes of them: the host key, ICE servers (resolved
//! once at start) and NetherNet's UDP port. See docs/DESIGN.md "NetherNet".

use std::fmt;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;

use acacia_nethernet::host::{IceConfig, TurnServer};
use acacia_nethernet::lan::DISCOVERY_PORT;
use acacia_nethernet::turn::{IceServer, Scheme, Transport};
use p384::ecdsa::SigningKey;

/// NetherNet (WebRTC) hosting. Direct connect is signaled over HTTP on the TCP side of the game
/// port, and its datagrams share the game's UDP socket unless `udp_ports` gives it one.
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
    /// STUN and TURN servers (`stun:`/`turn:` URLs; credentials for TURN). Answers then also carry
    /// the server's address as STUN sees it and a TURN relay, for a server behind NAT whose UDP
    /// port is not forwarded. UDP TURN only. None by default.
    pub ice_servers: Vec<IceServer>,
    /// UDP ports NetherNet's own socket may take (the first free one); `None` shares the game's.
    pub udp_ports: Option<PortRange>,
    /// Also serve signaling over HTTPS on the same port. Without it a TLS hello gets the alert that
    /// sends the client back to HTTP, as BDS does.
    pub tls: Option<Tls>,
}

impl NetherNet {
    pub fn new(host_key: SigningKey) -> Self {
        Self { host_key, public_ip: None, lan: None, ice_servers: Vec::new(), udp_ports: None, tls: None }
    }
}

/// PEM files for HTTPS signaling: the certificate chain, leaf first, and its private key. Clients
/// check it against the name they joined by, so an address joined by IP needs a certificate for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tls {
    pub cert_chain: PathBuf,
    pub key: PathBuf,
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

/// An inclusive UDP port range, written `19133` or `19133-19140` as dragonfly's `UDPPorts`. All
/// NetherNet peers share one socket on the first free port, so a range is only fallback; `0` lets
/// the system pick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortRange {
    pub min: u16,
    pub max: u16,
}

impl PortRange {
    pub fn new(min: u16, max: u16) -> Option<Self> {
        (min <= max).then_some(Self { min, max })
    }
}

impl fmt::Display for PortRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.min == self.max { write!(f, "{}", self.min) } else { write!(f, "{}-{}", self.min, self.max) }
    }
}

impl FromStr for PortRange {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        let port = |p: &str| p.trim().parse::<u16>().map_err(|_| format!("bad port {p:?}"));
        let (min, max) = match s.split_once('-') {
            Some((min, max)) => (port(min)?, port(max)?),
            None => (port(s)?, port(s)?),
        };
        Self::new(min, max).ok_or_else(|| format!("empty port range {s}"))
    }
}

/// The ICE servers as the host takes them: addresses of the socket's family, looked up now. A server
/// that does not resolve, or a TURN one over TCP or TLS, is left out with a warning.
pub(crate) async fn resolve_ice(servers: &[IceServer], ipv4: bool) -> IceConfig {
    let mut ice = IceConfig::default();
    for server in servers {
        for url in &server.urls {
            let authority = if url.host.contains(':') { format!("[{}]:{}", url.host, url.port) } else { format!("{}:{}", url.host, url.port) };
            if url.scheme != Scheme::Stun && (url.scheme == Scheme::Turns || url.transport != Transport::Udp) {
                tracing::warn!(%authority, "NetherNet: only UDP TURN is supported; server left out");
                continue;
            }
            let found = tokio::net::lookup_host(&authority).await.ok().and_then(|mut addrs| addrs.find(|addr| addr.is_ipv4() == ipv4));
            let Some(addr) = found else {
                tracing::warn!(%authority, "NetherNet: ICE server did not resolve; left out");
                continue;
            };
            match url.scheme {
                Scheme::Stun => ice.stun.push(addr),
                _ => ice.turn.push(TurnServer { addr, username: server.username.clone(), password: server.password.clone() }),
            }
        }
    }
    ice
}

/// Where answers say NetherNet is: `public_ip` or the address signaling reached, at NetherNet's port.
pub(crate) fn local_candidate(public_ip: Option<IpAddr>, reached: IpAddr, port: u16) -> SocketAddr {
    SocketAddr::new(public_ip.unwrap_or(reached), port)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_ranges_read_as_dragonflys() {
        assert_eq!("19133".parse(), Ok(PortRange { min: 19133, max: 19133 }));
        assert_eq!("19133-19140".parse(), Ok(PortRange { min: 19133, max: 19140 }));
        assert!("19140-19133".parse::<PortRange>().is_err() && "x".parse::<PortRange>().is_err());
        assert_eq!(PortRange::new(5, 6).unwrap().to_string(), "5-6");
    }

    #[tokio::test]
    async fn ice_servers_resolve_to_the_sockets_family() {
        let url = |u: &str| u.parse().unwrap();
        let server = IceServer { username: "u".into(), password: "p".into(), urls: vec![url("stun:127.0.0.1:3478"), url("turn:127.0.0.1:3479"), url("turns:127.0.0.1"), url("stun:[::1]:3478")] };
        let ice = resolve_ice(&[server], true).await;
        assert_eq!(ice.stun, ["127.0.0.1:3478".parse().unwrap()]);
        assert_eq!(ice.turn, [TurnServer { addr: "127.0.0.1:3479".parse().unwrap(), username: "u".into(), password: "p".into() }]);
    }
}
