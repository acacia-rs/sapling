//! What a source chunk generated, kept while its neighbours are generated. Any bound works:
//! entries are pure functions of the key, so a dropped one is only recomputed.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex};

pub struct SourceCache<K, V> {
    entries: Mutex<HashMap<K, Arc<V>>>,
    limit: usize,
}

impl<K: Hash + Eq + Copy, V> SourceCache<K, V> {
    pub fn new(limit: usize) -> Self {
        Self { entries: Mutex::new(HashMap::new()), limit }
    }

    /// `make` runs outside the lock, so two threads may both make a missing entry; the first kept wins.
    pub fn get(&self, key: K, make: impl FnOnce() -> V) -> Arc<V> {
        if let Some(value) = self.entries.lock().unwrap().get(&key) {
            return value.clone();
        }
        let value = Arc::new(make());
        let mut entries = self.entries.lock().unwrap();
        if entries.len() >= self.limit {
            entries.clear();
        }
        entries.entry(key).or_insert(value).clone()
    }
}
