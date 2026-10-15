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

    /// Entries kept, not counting dropped ones another cache still holds.
    pub fn len(&self) -> usize {
        let entries = self.entries.lock().unwrap();
        entries.current.len() + entries.old.len()
    }
}

/// A pure function of a column's coordinates, direct-mapped: a slot lost to a collision is recomputed.
/// A clone starts empty.
pub struct ColumnCache<V> {
    bits: u32,
    slots: Mutex<Vec<Option<((i32, i32), V)>>>,
}

impl<V: Clone> ColumnCache<V> {
    /// `1 << bits` slots.
    pub fn new(bits: u32) -> Self {
        Self { bits, slots: Mutex::new(vec![None; 1 << bits]) }
    }

    pub fn get(&self, x: i32, z: i32, compute: impl FnOnce() -> V) -> V {
        let mix = (x as u32).wrapping_mul(0x9E37_79B1).wrapping_add(z as u32).wrapping_mul(0x85EB_CA6B);
        let slot = (mix >> (32 - self.bits)) as usize;
        if let Some((key, value)) = &self.slots.lock().unwrap()[slot]
            && *key == (x, z)
        {
            return value.clone();
        }
        let value = compute();
        self.slots.lock().unwrap()[slot] = Some(((x, z), value.clone()));
        value
    }
}

impl<V: Clone> Clone for ColumnCache<V> {
    fn clone(&self) -> Self {
        Self::new(self.bits)
    }
}

impl<V> std::fmt::Debug for ColumnCache<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("ColumnCache")
    }
}
