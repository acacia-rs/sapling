//! Crash reports: what a shard that panicked or hung was doing, written as a text file to
//! `ShardConfig::crash_reports` (or logged without one). A panic's backtrace is caught by a panic
//! hook on shard threads, since `catch_unwind` only hands back the message.

use std::any::Any;
use std::backtrace::Backtrace;
use std::cell::RefCell;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Once;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::timing::Phase;

/// Shard threads are named this and their index (`Shard::spawn`).
pub(crate) const THREAD_PREFIX: &str = "sapling-shard-";

thread_local! {
    static LAST_PANIC: RefCell<Option<(String, String)>> = const { RefCell::new(None) };
}

/// Chains a hook that keeps a shard thread's panic location and backtrace for its report.
pub(crate) fn install_hook() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if std::thread::current().name().is_some_and(|name| name.starts_with(THREAD_PREFIX)) {
                let at = info.location().map_or_else(String::new, |at| at.to_string());
                LAST_PANIC.set(Some((at, Backtrace::force_capture().to_string())));
            }
            previous(info);
        }));
    });
}

/// The location and backtrace of this thread's last panic.
pub(crate) fn take_panic() -> (String, String) {
    LAST_PANIC.take().unwrap_or_else(|| (String::new(), "(not captured)".to_owned()))
}

pub(crate) fn panic_message(payload: &(dyn Any + Send)) -> String {
    match (payload.downcast_ref::<&str>(), payload.downcast_ref::<String>()) {
        (Some(message), _) => (*message).to_owned(),
        (_, Some(message)) => message.clone(),
        _ => "(not a string)".to_owned(),
    }
}

pub(crate) struct Report {
    /// "panic" or "hang".
    pub kind: &'static str,
    pub shard: usize,
    pub tick: u64,
    pub phase: Phase,
    pub message: String,
    pub location: String,
    pub backtrace: String,
    /// One line per world of the shard.
    pub worlds: Vec<String>,
    /// What became of the shard's worlds and players.
    pub saved: String,
}

impl Report {
    pub fn render(&self, unix_secs: u64) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "sapling crash report ({})", self.kind);
        let _ = writeln!(out, "time: {}", utc(unix_secs));
        let _ = writeln!(out, "version: sapling {}, Minecraft {} (protocol {})", env!("CARGO_PKG_VERSION"), acacia_proto::GAME_VERSION, acacia_proto::PROTOCOL_VERSION);
        let _ = writeln!(out, "shard: {} (tick {}, phase {:?})", self.shard, self.tick, self.phase);
        let _ = writeln!(out, "message: {}", self.message);
        if !self.location.is_empty() {
            let _ = writeln!(out, "at: {}", self.location);
        }
        let _ = writeln!(out, "saved: {}", self.saved);
        let _ = writeln!(out, "worlds:");
        for world in &self.worlds {
            let _ = writeln!(out, "  {world}");
        }
        let _ = writeln!(out, "backtrace:\n{}", self.backtrace);
        out
    }

    /// Writes the report into `dir` (made if missing), or logs it when there is none.
    pub fn file(&self, dir: Option<&Path>) -> Option<PathBuf> {
        let secs = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs());
        let text = self.render(secs);
        let Some(dir) = dir else {
            tracing::error!("{text}");
            return None;
        };
        let path = dir.join(format!("crash-{secs}-shard{}-{}.txt", self.shard, self.kind));
        // Whole or not at all, for whoever watches the folder.
        let partial = path.with_extension("part");
        match std::fs::create_dir_all(dir).and_then(|()| std::fs::write(&partial, &text)).and_then(|()| std::fs::rename(&partial, &path)) {
            Ok(()) => Some(path),
            Err(error) => {
                tracing::error!(%error, path = %path.display(), "the crash report could not be written:\n{text}");
                None
            }
        }
    }
}

/// `YYYY-MM-DD hh:mm:ss UTC` of Unix seconds (civil-from-days, proleptic Gregorian).
fn utc(secs: u64) -> String {
    let (days, rest) = ((secs / 86_400) as i64, secs % 86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02} UTC", rest / 3600, rest / 60 % 60, rest % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_names_the_time_the_shard_and_the_panic() {
        assert_eq!(utc(0), "1970-01-01 00:00:00 UTC");
        assert_eq!(utc(1_792_929_845), "2026-10-25 12:04:05 UTC");
        assert_eq!(utc(951_782_400), "2000-02-29 00:00:00 UTC");
        let report = Report {
            kind: "panic",
            shard: 2,
            tick: 77,
            phase: Phase::Handlers,
            message: "boom".into(),
            location: "src/x.rs:1:2".into(),
            backtrace: "frames".into(),
            worlds: vec!["arena (dimension 0, 4 columns, 1 player)".into()],
            saved: "yes".into(),
        };
        let text = report.render(0);
        for line in ["shard: 2 (tick 77, phase Handlers)", "message: boom", "at: src/x.rs:1:2", "  arena (dimension 0, 4 columns, 1 player)", "saved: yes"] {
            assert!(text.lines().any(|l| l == line), "{line:?} missing from\n{text}");
        }
        assert_eq!(panic_message(&*std::panic::catch_unwind(|| panic!("at {}", 3)).unwrap_err()), "at 3");
    }
}
