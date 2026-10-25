//! What a shard publishes about its ticks, for the watchdog and for metrics: atomics the shard
//! thread writes and any thread reads, so the game takes no lock (docs/DESIGN.md "Operations").

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

/// Ticks the published mean, longest and rate are taken over: five seconds at 20 Hz.
const WINDOW: usize = 100;

/// Nanoseconds from the first call in the process to `at`, plus one: 0 stands for none.
fn nanos(at: Instant) -> u64 {
    static EPOCH: OnceLock<Instant> = OnceLock::new();
    at.saturating_duration_since(*EPOCH.get_or_init(Instant::now)).as_nanos() as u64 + 1
}

/// Which part of its tick a shard is in (`Shard::tick`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Between ticks.
    Waiting,
    /// Taking what the network and the other shards delivered.
    Messages,
    /// The systems: movement, chunks, inventories.
    Systems,
    /// Block rules, then the plugin handlers.
    Handlers,
    /// Telling players about each other and sending the tick's packets.
    Sending,
}

impl Phase {
    const ALL: [Phase; 5] = [Phase::Waiting, Phase::Messages, Phase::Systems, Phase::Handlers, Phase::Sending];
}

/// One shard's published timing. [`crate::Shard::timing`] hands it out.
#[derive(Debug)]
pub struct ShardTiming {
    shard: usize,
    phase: AtomicU8,
    /// When the tick in progress began ([`nanos`]); 0 between ticks.
    began: AtomicU64,
    ticks: AtomicU64,
    mean_ns: AtomicU64,
    max_ns: AtomicU64,
    /// Ticks a second, times 1000.
    rate: AtomicU64,
}

/// A shard's tick times over the last five seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TickStats {
    pub shard: usize,
    /// Ticks run since the shard started.
    pub ticks: u64,
    pub mean: Duration,
    pub max: Duration,
    /// Ticks a second: 20 for a shard that keeps up.
    pub tps: f64,
}

impl ShardTiming {
    pub(crate) fn new(shard: usize) -> Self {
        let zero = || AtomicU64::new(0);
        Self { shard, phase: AtomicU8::new(0), began: zero(), ticks: zero(), mean_ns: zero(), max_ns: zero(), rate: zero() }
    }

    pub fn shard(&self) -> usize {
        self.shard
    }

    pub fn phase(&self) -> Phase {
        Phase::ALL[usize::from(self.phase.load(Relaxed)).min(Phase::ALL.len() - 1)]
    }

    pub fn stats(&self) -> TickStats {
        let ns = |value: &AtomicU64| Duration::from_nanos(value.load(Relaxed));
        let tps = self.rate.load(Relaxed) as f64 / 1000.0;
        TickStats { shard: self.shard, ticks: self.ticks.load(Relaxed), mean: ns(&self.mean_ns), max: ns(&self.max_ns), tps }
    }

    /// The tick in progress (counted from 0) and how long it has run; None between ticks.
    pub fn in_tick(&self, now: Instant) -> Option<(u64, Duration)> {
        let began = self.began.load(Relaxed);
        (began != 0).then(|| (self.ticks.load(Relaxed), Duration::from_nanos(nanos(now).saturating_sub(began))))
    }

    #[cfg(test)]
    pub(crate) fn begun_at(&self, at: Instant) {
        self.began.store(nanos(at), Relaxed);
    }
}

/// The shard thread's side: keeps the window and publishes into the [`ShardTiming`].
pub(crate) struct TickMeter {
    shared: Arc<ShardTiming>,
    began: Option<Instant>,
    starts: VecDeque<Instant>,
    lengths: VecDeque<Duration>,
}

impl TickMeter {
    pub fn new(shard: usize) -> Self {
        Self { shared: Arc::new(ShardTiming::new(shard)), began: None, starts: VecDeque::new(), lengths: VecDeque::new() }
    }

    pub fn shared(&self) -> Arc<ShardTiming> {
        self.shared.clone()
    }

    pub fn begin(&mut self) {
        let now = Instant::now();
        self.began = Some(now);
        push(&mut self.starts, now);
        self.shared.began.store(nanos(now), Relaxed);
        self.enter(Phase::Messages);
    }

    pub fn enter(&self, phase: Phase) {
        self.shared.phase.store(phase as u8, Relaxed);
    }

    pub fn end(&mut self) {
        let Some(began) = self.began.take() else { return };
        push(&mut self.lengths, began.elapsed());
        let shared = &self.shared;
        let total: Duration = self.lengths.iter().sum();
        shared.mean_ns.store((total / self.lengths.len() as u32).as_nanos() as u64, Relaxed);
        shared.max_ns.store(self.lengths.iter().max().map_or(0, |max| max.as_nanos() as u64), Relaxed);
        if let (Some(first), Some(last)) = (self.starts.front(), self.starts.back()) {
            let span = last.duration_since(*first).as_secs_f64();
            if span > 0.0 {
                shared.rate.store(((self.starts.len() - 1) as f64 / span * 1000.0) as u64, Relaxed);
            }
        }
        self.abandon();
        shared.ticks.fetch_add(1, Relaxed);
    }

    /// Leaves the tick in progress unfinished: the shard stops in it (a panic).
    pub fn abandon(&self) {
        self.shared.began.store(0, Relaxed);
        self.enter(Phase::Waiting);
    }
}

fn push<T>(window: &mut VecDeque<T>, value: T) {
    if window.len() == WINDOW {
        window.pop_front();
    }
    window.push_back(value);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tick_is_seen_while_it_runs_and_counted_when_it_ends() {
        let mut meter = TickMeter::new(3);
        let timing = meter.shared();
        assert_eq!((timing.in_tick(Instant::now()), timing.phase()), (None, Phase::Waiting));
        meter.begin();
        meter.enter(Phase::Handlers);
        std::thread::sleep(Duration::from_millis(5));
        let (tick, running) = timing.in_tick(Instant::now()).unwrap();
        assert_eq!((tick, timing.phase()), (0, Phase::Handlers));
        assert!(running >= Duration::from_millis(5));
        meter.end();
        let stats = timing.stats();
        assert_eq!((stats.shard, stats.ticks, timing.in_tick(Instant::now())), (3, 1, None));
        assert!(stats.max >= Duration::from_millis(5) && stats.mean == stats.max);
    }
}
