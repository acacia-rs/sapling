//! What a source chunk generated, kept while its neighbours are generated. Any bound works:
//! entries are pure functions of the key, so a dropped one is only recomputed.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex, OnceLock, Weak};

use crate::fxhash::FxBuild;

/// Filled by the first thread to ask; the others wait for it.
type Slot<V> = Arc<OnceLock<Arc<V>>>;

/// Two generations: a full current one becomes the old one, and an old entry used again moves
/// back, so a sweep keeps the neighbours it is still reading (clearing everything did not).
struct Generations<K, V> {
    current: HashMap<K, Slot<V>, FxBuild>,
    old: HashMap<K, Slot<V>, FxBuild>,
    /// Entries dropped from `old`: one another cache still holds (a staged chunk holds its carved
    /// terrain) is taken back instead of made again, at no memory cost.
    dropped: HashMap<K, Weak<V>, FxBuild>,
}

pub struct SourceCache<K, V> {
    entries: Mutex<Generations<K, V>>,
    /// Entries per generation.
    limit: usize,
}

impl<K: Hash + Eq + Copy, V> SourceCache<K, V> {
    /// Holds between `limit / 2` and `limit` entries.
    pub fn new(limit: usize) -> Self {
        let entries = Generations { current: HashMap::default(), old: HashMap::default(), dropped: HashMap::default() };
        Self { entries: Mutex::new(entries), limit: (limit / 2).max(1) }
    }

    /// `make` runs outside the lock and once per kept entry: threads generating neighbouring chunks ask
    /// for the same sources at the same time.
    pub fn get(&self, key: K, make: impl FnOnce() -> V) -> Arc<V> {
        let slot = {
            let mut entries = self.entries.lock().unwrap();
            match entries.current.get(&key) {
                Some(slot) => slot.clone(),
                None => {
                    let slot = match entries.old.remove(&key) {
                        Some(slot) => slot,
                        None => entries.dropped.remove(&key).and_then(|held| held.upgrade()).map_or_else(Slot::default, |value| Arc::new(OnceLock::from(value))),
                    };
                    if entries.current.len() >= self.limit {
                        let current = std::mem::take(&mut entries.current);
                        let old = std::mem::replace(&mut entries.old, current);
                        entries.dropped.retain(|_, held| held.strong_count() > 0);
                        entries.dropped.extend(old.into_iter().filter_map(|(key, slot)| slot.get().map(|value| (key, Arc::downgrade(value)))));
                    }
                    entries.current.insert(key, slot.clone());
                    slot
                }
            }
        };
        slot.get_or_init(|| Arc::new(make())).clone()
    }
}
