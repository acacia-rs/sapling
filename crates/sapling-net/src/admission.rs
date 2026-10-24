//! Per-address limits on new connections, checked as each transport comes up and before its login
//! costs a key pair. acacia caps peers and handshake time for the whole socket
//! (research/acacia-server-gaps.md R7); what one address may hold and how often it may try is ours.

use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};

/// Loopback addresses are exempt: a proxy or a test on the same machine speaks for many players.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admission {
    /// Connections one address may hold at once, logging in or playing.
    pub max_per_ip: usize,
    /// Connections one address may open per `window`.
    pub attempts_per_window: usize,
    pub window: Duration,
}

impl Default for Admission {
    fn default() -> Self {
        Self { max_per_ip: 8, attempts_per_window: 10, window: Duration::from_secs(10) }
    }
}

impl Admission {
    /// No limits, for load tests driving many clients from one machine.
    pub fn off() -> Self {
        Self { max_per_ip: usize::MAX, attempts_per_window: usize::MAX, window: Duration::ZERO }
    }
}

/// Why a connection was dropped before its login began.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Turned {
    TooMany,
    TooOften,
}

pub(crate) struct Throttle {
    cfg: Admission,
    /// When each address's current window began and the connections it opened in it.
    windows: HashMap<IpAddr, (Instant, usize)>,
    pruned: Option<Instant>,
}

impl Throttle {
    pub fn new(cfg: Admission) -> Self {
        Self { cfg, windows: HashMap::new(), pruned: None }
    }

    /// Counts a new connection from `ip`, which already holds `open`.
    pub fn admit(&mut self, ip: IpAddr, open: usize, now: Instant) -> Result<(), Turned> {
        if ip.is_loopback() || ip.is_unspecified() {
            return Ok(());
        }
        self.prune(now);
        if open >= self.cfg.max_per_ip {
            return Err(Turned::TooMany);
        }
        let (began, attempts) = self.windows.entry(ip).or_insert((now, 0));
        if now.duration_since(*began) >= self.cfg.window {
            (*began, *attempts) = (now, 0);
        }
        *attempts += 1;
        if *attempts > self.cfg.attempts_per_window { Err(Turned::TooOften) } else { Ok(()) }
    }

    /// Forgets windows that ended, once a window, so the map holds only addresses seen lately.
    fn prune(&mut self, now: Instant) {
        if self.pruned.is_some_and(|pruned| now.duration_since(pruned) < self.cfg.window) {
            return;
        }
        self.pruned = Some(now);
        let window = self.cfg.window;
        self.windows.retain(|_, (began, _)| now.duration_since(*began) < window);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_address_is_held_to_its_connections_and_its_rate() {
        let mut throttle = Throttle::new(Admission { max_per_ip: 2, attempts_per_window: 3, window: Duration::from_secs(10) });
        let (ip, other, start) = ("203.0.113.7".parse().unwrap(), "198.51.100.1".parse().unwrap(), Instant::now());
        assert_eq!(throttle.admit(ip, 2, start), Err(Turned::TooMany));
        let tries: Vec<_> = (0..4).map(|_| throttle.admit(ip, 0, start)).collect();
        assert_eq!(tries, [Ok(()), Ok(()), Ok(()), Err(Turned::TooOften)]);
        assert_eq!(throttle.admit(other, 0, start), Ok(()), "each address has its own window");
        assert_eq!(throttle.admit(ip, 0, start + Duration::from_secs(10)), Ok(()), "a new window");
        assert_eq!(throttle.windows.len(), 1, "the other address's window ended and was forgotten");
        for _ in 0..20 {
            assert_eq!(throttle.admit("127.0.0.1".parse().unwrap(), 50, start), Ok(()));
        }
        let mut off = Throttle::new(Admission::off());
        assert!((0..100).all(|open| off.admit(ip, open, start).is_ok()));
    }
}
