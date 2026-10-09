//! What a source chunk generated, kept while its neighbours are generated. Any bound works:
//! entries are pure functions of the key, so a dropped one is only recomputed.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex, OnceLock};

/// Filled by the first thread to ask; the others wait for it.
type Slot<V> = Arc<OnceLock<Arc<V>>>;

/// Two generations: a full current one becomes the old one, and an old entry used again moves
/// back, so a sweep keeps the neighbours it is still reading (clearing everything did not).
struct Generations<K, V> {
    current: HashMap<K, Slot<V>>,
    old: HashMap<K, Slot<V>>,
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

    /// `make` runs outside the lock and once per kept entry: threads generating neighbouring chunks ask
    /// for the same sources at the same time.
    pub fn get(&self, key: K, make: impl FnOnce() -> V) -> Arc<V> {
        let slot = {
            let mut entries = self.entries.lock().unwrap();
            match entries.current.get(&key) {
                Some(slot) => slot.clone(),
                None => {
                    let slot = entries.old.remove(&key).unwrap_or_default();
                    if entries.current.len() >= self.limit {
                        entries.old = std::mem::take(&mut entries.current);
                    }
                    entries.current.insert(key, slot.clone());
                    slot
                }
            }
        };
        slot.get_or_init(|| Arc::new(make())).clone()
    }
}
