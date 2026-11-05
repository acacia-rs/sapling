//! Which BDS build generation follows where Windows and Linux BDS differ: Windows by default (sapling matches
//! Windows BDS for what a world contains), Linux only to replay a canon made on the Linux BDS (`env.txt` beside
//! it: `SAPLING_ORACLE_LINUX`). The differences: docs/research/worldgen-bds-linux-canons.md.

use std::sync::atomic::{AtomicBool, Ordering};

static LINUX: AtomicBool = AtomicBool::new(false);

/// Process-wide: generate as the Linux BDS does from here on.
#[doc(hidden)]
pub fn follow_linux_bds() {
    LINUX.store(true, Ordering::Relaxed);
}

pub(crate) fn linux() -> bool {
    LINUX.load(Ordering::Relaxed)
}
