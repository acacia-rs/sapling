//! Probe time under `ACACIA_TICK_SCALE`: against a drill server ticking that many times faster
//! (drill_server.rs) a probe watches for proportionally less wall time, and prints times in
//! 20-ticks-a-second milliseconds so its captures line up with BDS's.

use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// `ACACIA_TICK_SCALE` as the bots read it (acacia-bot's cadence.rs): anything but a finite positive number is 1.
pub fn tick_scale() -> f64 {
    static SCALE: OnceLock<f64> = OnceLock::new();
    *SCALE.get_or_init(|| std::env::var("ACACIA_TICK_SCALE").ok().and_then(|v| v.parse().ok()).filter(|s: &f64| s.is_finite() && *s > 0.0).unwrap_or(1.0))
}

/// `d` of 20-ticks-a-second time as wall time; exactly `d` at scale 1.
pub fn scaled(d: Duration) -> Duration {
    if tick_scale() == 1.0 { d } else { d.div_f64(tick_scale()) }
}

/// Prints when the drill proper starts, in Unix milliseconds: tools/sapling-drill.sh only counts the
/// server falling behind after it (joining a debug build lags harmlessly).
pub fn mark_start() {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    println!("## drill starts at {}", now.as_millis());
}

/// Milliseconds of 20-ticks-a-second time since `start`.
pub fn tick_ms(start: Instant) -> u128 {
    if tick_scale() == 1.0 { start.elapsed().as_millis() } else { start.elapsed().mul_f64(tick_scale()).as_millis() }
}
