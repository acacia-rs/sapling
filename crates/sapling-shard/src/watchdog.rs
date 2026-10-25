//! The tick watchdog: a thread that reads each shard's [`ShardTiming`] and reports a tick that runs
//! long (slow), one that seems stuck (hung, with the shard and the phase it is in, no stack) and,
//! if asked, aborts the process when one stays stuck. The shards take no lock for it.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::crash::Report;
use crate::timing::ShardTiming;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Watchdog {
    /// A tick that runs longer is logged as slow.
    pub slow: Duration,
    /// A tick that runs longer is logged as hung (BDS's script watchdog says 10 s).
    pub hung: Duration,
    /// A tick that runs longer aborts the process, after a crash report: the shard cannot be
    /// stopped or saved from outside. None (default): never.
    pub abort: Option<Duration>,
}

impl Default for Watchdog {
    fn default() -> Self {
        Self { slow: Duration::from_secs(1), hung: Duration::from_secs(10), abort: None }
    }
}

/// How bad a tick has got.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Level {
    Fine,
    Slow,
    Hung,
    Abort,
}

/// A level a shard's tick newly reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Alarm {
    pub shard: usize,
    pub tick: u64,
    pub running: Duration,
    pub level: Level,
}

/// The watchdog's memory: the worst level each shard's current tick has been reported at.
pub(crate) struct Watch {
    cfg: Watchdog,
    shards: Vec<Arc<ShardTiming>>,
    reported: Vec<(u64, Level)>,
}

impl Watch {
    pub fn new(cfg: Watchdog, shards: Vec<Arc<ShardTiming>>) -> Self {
        let reported = vec![(u64::MAX, Level::Fine); shards.len()];
        Self { cfg, shards, reported }
    }

    fn level(&self, running: Duration) -> Level {
        match running {
            r if self.cfg.abort.is_some_and(|abort| r >= abort) => Level::Abort,
            r if r >= self.cfg.hung => Level::Hung,
            r if r >= self.cfg.slow => Level::Slow,
            _ => Level::Fine,
        }
    }

    /// The ticks that got worse since the last scan.
    pub fn scan(&mut self, now: Instant) -> Vec<Alarm> {
        let mut alarms = Vec::new();
        for (index, timing) in self.shards.iter().enumerate() {
            let Some((tick, running)) = timing.in_tick(now) else { continue };
            let level = self.level(running);
            let (seen_tick, seen) = &mut self.reported[index];
            if *seen_tick != tick {
                (*seen_tick, *seen) = (tick, Level::Fine);
            }
            if level > *seen {
                *seen = level;
                alarms.push(Alarm { shard: timing.shard(), tick, running, level });
            }
        }
        alarms
    }

    fn timing(&self, shard: usize) -> Option<&ShardTiming> {
        self.shards.iter().find(|timing| timing.shard() == shard).map(Arc::as_ref)
    }
}

/// The running watchdog; dropping it stops the thread.
pub struct WatchdogHandle {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Watchdog {
    /// Watches `shards` ([`crate::Shard::timing`]) on a thread of its own; a hang that aborts
    /// writes its report into `reports`.
    pub fn spawn(self, shards: Vec<Arc<ShardTiming>>, reports: Option<PathBuf>) -> WatchdogHandle {
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let every = (self.slow / 4).clamp(Duration::from_millis(10), Duration::from_millis(250));
        let mut watch = Watch::new(self, shards);
        let run = move || {
            while !stopped.load(Ordering::Relaxed) {
                for alarm in watch.scan(Instant::now()) {
                    raise(&watch, alarm, reports.as_deref());
                }
                std::thread::park_timeout(every);
            }
        };
        let thread = std::thread::Builder::new().name("sapling-watchdog".into()).spawn(run).expect("spawning the watchdog thread");
        WatchdogHandle { stop, thread: Some(thread) }
    }
}

fn raise(watch: &Watch, alarm: Alarm, reports: Option<&std::path::Path>) {
    let Alarm { shard, tick, running, level } = alarm;
    let phase = watch.timing(shard).map(ShardTiming::phase);
    let ms = running.as_millis();
    match level {
        Level::Fine => {}
        Level::Slow => tracing::warn!(shard, tick, ?phase, ms, "a tick is running long"),
        Level::Hung => tracing::error!(shard, tick, ?phase, ms, "a shard seems hung"),
        Level::Abort => {
            tracing::error!(shard, tick, ?phase, ms, "a shard is stuck: aborting");
            let message = format!("tick {tick} ran {ms} ms without ending");
            let backtrace = "(not taken: the shard thread is stuck)".to_owned();
            let saved = "no: the stuck shard cannot be saved from outside".to_owned();
            let phase = phase.unwrap_or(crate::timing::Phase::Waiting);
            Report { kind: "hang", shard, tick, phase, message, location: String::new(), backtrace, worlds: Vec::new(), saved }.file(reports);
            std::process::abort();
        }
    }
}

impl Drop for WatchdogHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            thread.thread().unpark();
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timing::TickMeter;

    #[test]
    fn a_long_tick_is_reported_once_per_level() {
        let cfg = Watchdog { slow: Duration::from_millis(100), hung: Duration::from_secs(2), abort: Some(Duration::from_secs(5)) };
        let (mut busy, idle) = (TickMeter::new(0), TickMeter::new(1));
        let mut watch = Watch::new(cfg, vec![busy.shared(), idle.shared()]);
        let start = Instant::now();
        busy.begin();
        busy.shared().begun_at(start);
        let at = |ms: u64| start + Duration::from_millis(ms);
        assert!(watch.scan(at(50)).is_empty());
        let levels = |alarms: Vec<Alarm>| alarms.into_iter().map(|a| (a.shard, a.tick, a.level)).collect::<Vec<_>>();
        assert_eq!(levels(watch.scan(at(150))), [(0, 0, Level::Slow)]);
        assert!(watch.scan(at(900)).is_empty(), "reported once");
        assert_eq!(levels(watch.scan(at(2500))), [(0, 0, Level::Hung)]);
        assert_eq!(levels(watch.scan(at(6000))), [(0, 0, Level::Abort)]);
        // The next tick starts afresh.
        busy.end();
        busy.begin();
        busy.shared().begun_at(at(6000));
        assert_eq!(levels(watch.scan(at(6200))), [(0, 1, Level::Slow)]);
    }
}
