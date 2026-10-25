//! Running a server: `server.properties`, idle kicks, compression and packet limits, the tick
//! watchdog, crash reports and tick metrics (docs/DESIGN.md "Operations").

use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use sapling_net::{Compression, NetConfig, PacketLimits};
use sapling_shard::{Shard, ShardTiming, TickStats, Watchdog, WatchdogHandle};

use super::properties::{ConfigError, ServerProperties};
use super::{Server, ServerBuilder, StartError};

/// What the builder keeps for [`Watching`].
#[derive(Default)]
pub(crate) struct Ops {
    watchdog: Option<Watchdog>,
    metrics: Metrics,
    allow_list: Option<bool>,
}

/// What a running server keeps of them.
pub(crate) struct Watching {
    metrics: Metrics,
    _watchdog: Option<WatchdogHandle>,
}

impl Ops {
    /// Sets `allow-list` on the access lists; turning it on without lists is refused.
    pub(crate) fn apply_allow_list(&self, net: &NetConfig) -> Result<(), StartError> {
        match (self.allow_list, &net.access) {
            (Some(on), Some(lists)) => lists.set_allowlist_enabled(on),
            (Some(true), None) => return Err(StartError::NoAllowlist),
            _ => {}
        }
        Ok(())
    }

    /// Publishes the shards' timing and starts the watchdog over them.
    pub(crate) fn watch(self, shards: &[Shard], crash_reports: Option<PathBuf>) -> Watching {
        let timings: Vec<Arc<ShardTiming>> = shards.iter().map(Shard::timing).collect();
        let _ = self.metrics.0.set(timings.clone());
        Watching { metrics: self.metrics, _watchdog: self.watchdog.map(|watchdog| watchdog.spawn(timings, crash_reports)) }
    }
}

impl ServerBuilder {
    /// Applies a `server.properties` file ([`ServerProperties`]); a setting made after this call
    /// wins over the file's.
    pub fn config_file(self, path: impl AsRef<std::path::Path>) -> Result<Self, ConfigError> {
        Ok(self.properties(&ServerProperties::load(path)?))
    }

    /// Applies what a `server.properties` set. Its `level-name` names the world (not the folder,
    /// which the binary opens) and its keys without a setting are logged as ignored.
    pub fn properties(mut self, props: &ServerProperties) -> Self {
        for key in &props.ignored {
            tracing::warn!(key, "server.properties: sapling has no setting for this key; ignored");
        }
        if let Some(name) = &props.server_name {
            self = self.name(name.clone());
        }
        if let Some(level) = &props.level_name {
            (self.net.motd.sub_name, self.shard.world_name) = (level.clone(), level.clone());
        }
        self.shard.game_mode = props.gamemode.unwrap_or(self.shard.game_mode);
        self.shard.difficulty = props.difficulty.unwrap_or(self.shard.difficulty);
        self.net.max_players = props.max_players.unwrap_or(self.net.max_players);
        self.net.online_mode = props.online_mode.unwrap_or(self.net.online_mode);
        self.ops.allow_list = props.allow_list.or(self.ops.allow_list);
        if let Some(ip) = props.server_ip {
            self.addr.set_ip(ip);
        }
        self.addr.set_port(props.server_port.unwrap_or(self.addr.port()));
        self.shard.max_view_radius = props.view_distance.unwrap_or(self.shard.max_view_radius);
        if let Some(minutes) = props.player_idle_timeout {
            self = self.idle_timeout(Duration::from_secs(u64::from(minutes) * 60));
        }
        self.net.compression = props.compression_algorithm.unwrap_or(self.net.compression);
        self.net.compression_threshold = props.compression_threshold.unwrap_or(self.net.compression_threshold);
        let check = &mut self.shard.movement_check;
        check.threshold = props.position_threshold.unwrap_or(check.threshold);
        check.correction_delay = props.correction_delay.unwrap_or(check.correction_delay);
        self
    }

    /// The farthest a client may see, in chunks (default 8; BDS's `view-distance`).
    pub fn view_distance(mut self, chunks: i32) -> Self {
        self.shard.max_view_radius = chunks;
        self
    }

    /// Kicks a player who neither moves, turns nor acts for this long (`player-idle-timeout`);
    /// zero, the default, never does.
    pub fn idle_timeout(mut self, timeout: Duration) -> Self {
        self.shard.idle_timeout = (!timeout.is_zero()).then_some(timeout);
        self
    }

    /// How batches are compressed and from what length (default zlib from 1 byte, as BDS).
    pub fn compression(mut self, algorithm: Compression, threshold: u16) -> Self {
        (self.net.compression, self.net.compression_threshold) = (algorithm, threshold);
        self
    }

    /// What one client may send a second (default [`PacketLimits::default`]; loopback exempt).
    pub fn packet_limits(mut self, limits: PacketLimits) -> Self {
        self.net.packet_limits = limits;
        self
    }

    /// Watches every shard's tick from a thread of its own ([`Watchdog`]).
    pub fn watchdog(mut self, watchdog: Watchdog) -> Self {
        self.ops.watchdog = Some(watchdog);
        self
    }

    /// Where a shard that panics, or a hang the watchdog aborts on, writes its crash report
    /// (made if missing). Without it the report is logged. A panicking shard saves its worlds and
    /// players and stops either way; the others go on.
    pub fn crash_reports(mut self, dir: impl Into<PathBuf>) -> Self {
        self.shard.crash_reports = Some(dir.into());
        self
    }

    /// Fills `metrics` once the server starts, so a handler built before it can hold them.
    pub fn metrics(mut self, metrics: &Metrics) -> Self {
        self.ops.metrics = metrics.clone();
        self
    }
}

impl Server {
    /// Each shard's tick times over the last five seconds.
    pub fn tick_stats(&self) -> Vec<TickStats> {
        self.watching.metrics.stats()
    }

    pub fn metrics(&self) -> Metrics {
        self.watching.metrics.clone()
    }
}

/// Tick times of a server's shards, readable from any thread without a lock; empty until the
/// server starts ([`ServerBuilder::metrics`]).
#[derive(Debug, Clone, Default)]
pub struct Metrics(Arc<OnceLock<Vec<Arc<ShardTiming>>>>);

impl Metrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn stats(&self) -> Vec<TickStats> {
        self.0.get().map_or_else(Vec::new, |shards| shards.iter().map(|shard| shard.stats()).collect())
    }

    /// The stats in Prometheus' text format, for a scrape endpoint the binary serves.
    pub fn prometheus(&self) -> String {
        let stats = self.stats();
        let mut out = String::new();
        let gauges: [(&str, &str, fn(&TickStats) -> f64); 4] = [
            ("sapling_tick_mean_seconds", "gauge", |s| s.mean.as_secs_f64()),
            ("sapling_tick_max_seconds", "gauge", |s| s.max.as_secs_f64()),
            ("sapling_ticks_per_second", "gauge", |s| s.tps),
            ("sapling_ticks_total", "counter", |s| s.ticks as f64),
        ];
        for (name, kind, value) in gauges {
            let _ = writeln!(out, "# TYPE {name} {kind}");
            for shard in &stats {
                let _ = writeln!(out, "{name}{{shard=\"{}\"}} {}", shard.shard, value(shard));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sapling_api::GameMode;

    #[test]
    fn a_properties_file_reaches_the_settings_and_code_after_it_wins() {
        let props = ServerProperties::parse("server-name=Hub\nlevel-name=Spawn\ngamemode=creative\nserver-port=19200\nplayer-idle-timeout=2\nview-distance=12\ncompression-algorithm=snappy").unwrap();
        let builder = Server::builder().properties(&props).view_distance(10);
        assert_eq!((builder.net.motd.name.as_str(), builder.net.motd.sub_name.as_str(), builder.shard.world_name.as_str()), ("Hub", "Spawn", "Spawn"));
        assert_eq!((builder.shard.game_mode, builder.addr.port(), builder.shard.max_view_radius), (GameMode::Creative, 19200, 10));
        assert_eq!((builder.shard.idle_timeout, builder.net.compression), (Some(Duration::from_secs(120)), Compression::Snappy));
        let allow = ServerProperties::parse("allow-list=true").unwrap();
        assert!(matches!(Server::builder().properties(&allow).ops.apply_allow_list(&NetConfig::default()), Err(StartError::NoAllowlist)));
        assert!(Metrics::new().stats().is_empty() && Metrics::new().prometheus().lines().all(|line| line.starts_with('#')));
    }
}
