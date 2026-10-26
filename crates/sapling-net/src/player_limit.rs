use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// How many players may be online at once: a handle the listener reads at each login and
/// advertisement, so a change (`/setmaxplayers`) holds from the next one. Clones share it. The
/// listener makes room for the ceiling it starts with: a limit set above that is not reached.
#[derive(Debug, Clone)]
pub struct PlayerLimit(Arc<[AtomicUsize; 2]>);

const MAX: usize = 0;
const CEILING: usize = 1;

impl PlayerLimit {
    pub fn new(max: usize) -> Self {
        Self(Arc::new([AtomicUsize::new(max), AtomicUsize::new(max)]))
    }

    pub fn get(&self) -> usize {
        self.0[MAX].load(Ordering::Relaxed)
    }

    /// The most the limit may be set to.
    pub fn ceiling(&self) -> usize {
        self.0[CEILING].load(Ordering::Relaxed)
    }

    /// Changes the limit, up to the ceiling.
    pub fn set(&self, max: usize) {
        self.0[MAX].store(max.min(self.ceiling()), Ordering::Relaxed);
    }

    /// Sets the limit and its ceiling, for every clone; before the listener starts.
    pub fn reset(&self, max: usize) {
        self.0[CEILING].store(max, Ordering::Relaxed);
        self.0[MAX].store(max, Ordering::Relaxed);
    }
}

impl Default for PlayerLimit {
    fn default() -> Self {
        Self::new(100)
    }
}
