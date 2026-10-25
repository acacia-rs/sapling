//! Per-session packet rates: how many game packets one client may send a second, of all types and
//! of the types a client has no reason to send often. What exceeds a limit is dropped and makes
//! the second a strike; `strikes` seconds in a row close the connection. Loopback sessions are not
//! metered: drills and tests run the game clock faster (examples/drill_server.rs), and a proxy on
//! the same machine is metered by its own front end.

use std::time::{Duration, Instant};

use acacia_proto::Packet;
use acacia_proto::packets::{Animate, BookEdit, CommandRequest, Emote, Text};

const SECOND: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketLimits {
    /// Game packets a second, of all types.
    pub per_second: u32,
    /// A packet id and how many of it a second.
    pub per_type: Vec<(u32, u32)>,
    /// Seconds over a limit in a row before the connection is closed.
    pub strikes: u32,
}

impl Default for PacketLimits {
    /// A client sends PlayerAuthInput 20 times a second and little beside it; these leave it
    /// ample room.
    fn default() -> Self {
        let per_type = vec![(Text::ID, 10), (CommandRequest::ID, 10), (Animate::ID, 40), (Emote::ID, 5), (BookEdit::ID, 10)];
        Self { per_second: 400, per_type, strikes: 5 }
    }
}

impl PacketLimits {
    /// No limits, for load tests.
    pub fn off() -> Self {
        Self { per_second: u32::MAX, per_type: Vec::new(), strikes: u32::MAX }
    }
}

/// What becomes of one packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verdict {
    Pass,
    Drop,
    Close,
}

/// One session's counts in the current second.
pub(crate) struct Meter {
    began: Instant,
    total: u32,
    /// Counts by the index of the type in `PacketLimits::per_type`.
    by_type: Vec<u32>,
    over: bool,
    strikes: u32,
}

impl Meter {
    pub fn new(now: Instant) -> Self {
        Self { began: now, total: 0, by_type: Vec::new(), over: false, strikes: 0 }
    }

    pub fn count(&mut self, limits: &PacketLimits, id: u32, now: Instant) -> Verdict {
        let elapsed = now.duration_since(self.began);
        if elapsed >= SECOND {
            // A second without excess, or a quiet one between, ends the run of strikes.
            if !self.over || elapsed >= 2 * SECOND {
                self.strikes = 0;
            }
            (self.began, self.total, self.over) = (now, 0, false);
            self.by_type.clear();
        }
        self.by_type.resize(limits.per_type.len(), 0);
        self.total = self.total.saturating_add(1);
        let mut within = self.total <= limits.per_second;
        if let Some(index) = limits.per_type.iter().position(|(of, _)| *of == id) {
            self.by_type[index] += 1;
            within &= self.by_type[index] <= limits.per_type[index].1;
        }
        if within {
            return Verdict::Pass;
        }
        if !self.over {
            self.over = true;
            self.strikes += 1;
        }
        if self.strikes >= limits.strikes { Verdict::Close } else { Verdict::Drop }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INPUT: u32 = 144;

    fn limits() -> PacketLimits {
        PacketLimits { per_second: 30, per_type: vec![(Text::ID, 2)], strikes: 3 }
    }

    /// The verdicts of `n` packets of `id` sent at `at`.
    fn burst(meter: &mut Meter, id: u32, n: usize, at: Instant) -> Vec<Verdict> {
        (0..n).map(|_| meter.count(&limits(), id, at)).collect()
    }

    #[test]
    fn excess_is_dropped_and_a_sustained_flood_closes_the_session() {
        let (start, mut meter) = (Instant::now(), Meter::new(Instant::now()));
        assert!(burst(&mut meter, INPUT, 30, start).iter().all(|v| *v == Verdict::Pass));
        assert_eq!(burst(&mut meter, INPUT, 2, start), [Verdict::Drop, Verdict::Drop]);
        // The second ran over; a quiet one after it forgives it.
        let later = start + Duration::from_secs(3);
        assert_eq!(burst(&mut meter, Text::ID, 3, later), [Verdict::Pass, Verdict::Pass, Verdict::Drop]);
        assert_eq!(meter.strikes, 1, "the strike of the first second was forgiven");
        let at = |s: u64| later + Duration::from_secs(s);
        assert_eq!(burst(&mut meter, Text::ID, 3, at(1)).last(), Some(&Verdict::Drop));
        assert_eq!(burst(&mut meter, Text::ID, 3, at(2)).last(), Some(&Verdict::Close), "three seconds over in a row");
    }

    #[test]
    fn a_second_within_the_limits_ends_the_strikes() {
        let start = Instant::now();
        let mut meter = Meter::new(start);
        let at = |ms: u64| start + Duration::from_millis(ms);
        burst(&mut meter, Text::ID, 3, at(0));
        burst(&mut meter, Text::ID, 3, at(1000));
        assert_eq!(burst(&mut meter, Text::ID, 2, at(2000)), [Verdict::Pass, Verdict::Pass]);
        assert_eq!(burst(&mut meter, Text::ID, 3, at(3000)).last(), Some(&Verdict::Drop), "strikes began again");
        let mut off = Meter::new(start);
        assert!((0..10_000).all(|_| off.count(&PacketLimits::off(), Text::ID, start) == Verdict::Pass));
    }
}
