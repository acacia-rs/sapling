//! A structure start map: a `SourceCache` that also counts the entries it made, so that a probe can
//! tell an evicted start built again from one kept (examples/walk.rs). The count lives here, not in
//! `SourceCache`, whose hot callers measurably paid for it.

use std::hash::Hash;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

use crate::cache::SourceCache;

pub(super) struct StartCache<K, V> {
    cache: SourceCache<K, V>,
    made: AtomicUsize,
}

impl<K: Hash + Eq + Copy, V> StartCache<K, V> {
    pub(super) fn new(limit: usize) -> Self {
        Self { cache: SourceCache::new(limit), made: AtomicUsize::new(0) }
    }

    pub(super) fn get(&self, key: K, make: impl FnOnce() -> V) -> Arc<V> {
        self.cache.get(key, || {
            self.made.fetch_add(1, Relaxed);
            make()
        })
    }

    /// Entries kept and made.
    pub(super) fn counts(&self) -> (usize, usize) {
        (self.cache.len(), self.made.load(Relaxed))
    }
}
