//! What a source chunk generated, kept while its neighbours are generated. Any bound works:
//! entries are pure functions of the key, so a dropped one is only recomputed.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex};

/// Two generations: a full current one becomes the old one, and an old entry used again moves
/// back, so a sweep keeps the neighbours it is still reading (clearing everything did not).
struct Generations<K, V> {
    current: HashMap<K, Arc<V>>,
    old: HashMap<K, Arc<V>>,
}

pub struct SourceCache<K, V> {
    entries: Mutex<Generations<K, V>>,
    /// Entries per generation.
    limit: usize,
}

impl<K: Hash + Eq + Copy, V> SourceCache<K, V> {
    /// Holds between `limit / 2` and `limit` entries.
    pub fn new(limit: usize) -> Self {
        Self { entries: Mutex::new(Generations { current: HashMap::new(), old: HashMap::new() }), limit: (limit / 2).max(1) }
    }

    /// `make` runs outside the lock, so two threads may both make a missing entry; the first kept wins.
    pub fn get(&self, key: K, make: impl FnOnce() -> V) -> Arc<V> {
        {
            let mut entries = self.entries.lock().unwrap();
            if let Some(value) = entries.current.get(&key) {
                return value.clone();
            }
            if let Some(value) = entries.old.remove(&key) {
                return self.insert(&mut entries, key, value);
            }
        }
        let value = Arc::new(make());
        let mut entries = self.entries.lock().unwrap();
        self.insert(&mut entries, key, value)
    }

    fn insert(&self, entries: &mut Generations<K, V>, key: K, value: Arc<V>) -> Arc<V> {
        if entries.current.len() >= self.limit {
            entries.old = std::mem::take(&mut entries.current);
        }
        entries.current.entry(key).or_insert(value).clone()
    }
}
