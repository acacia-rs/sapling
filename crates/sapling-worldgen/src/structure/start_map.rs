//! The order a `StructureFeature` visits its starts in (`foreachIntersectingStructureStart`
//! 141efb070): the list of its start map, an MSVC `std::unordered_map<ChunkPos, start>` that
//! `createBlueprints` (141ef9dc0) fills as chunks load. It follows the load order, so only a
//! replayed run has it: docs/research/worldgen-bds-order.md, "Start order".

use std::collections::HashSet;

/// `createBlueprints` scans this many chunks either way (feature +0xc0).
const RADIUS: i32 = 8;
const FIRST_BUCKETS: usize = 8;

#[derive(Default)]
pub struct StartMap {
    /// Chunks a scan has tested (the feature's set at +0x80).
    visited: HashSet<(i32, i32)>,
    /// The map's list (+0x40) with each key's hash; a bucket's entries are adjacent.
    list: Vec<((i32, i32), u64)>,
    buckets: usize,
}

/// FNV-1a over an int's bytes, low byte first.
fn fnv(value: i32) -> u64 {
    value.to_le_bytes().iter().fold(0xcbf2_9ce4_8422_2325, |h, &byte| (h ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3))
}

/// `std::hash<ChunkPos>`: the two ints' FNV hashes combined as boost's `hash_combine` does.
fn hash((x, z): (i32, i32)) -> u64 {
    let seed = fnv(x).wrapping_add(0x9e37_79b9);
    fnv(z).wrapping_add(0x9e37_79b9).wrapping_add(seed >> 2).wrapping_add(seed << 6) ^ seed
}

/// MSVC's `_Insert_new_node` and `_Forced_rehash` alike: in front of its bucket's entries, or at
/// the list's end when the bucket is empty.
fn place(list: &mut Vec<((i32, i32), u64)>, entry: ((i32, i32), u64), buckets: usize) {
    let mask = buckets as u64 - 1;
    match list.iter().position(|other| other.1 & mask == entry.1 & mask) {
        Some(front) => list.insert(front, entry),
        None => list.push(entry),
    }
}

impl StartMap {
    /// A chunk loads: every chunk within 8 not scanned before is tested, x outer, and its start filed.
    pub fn load(&mut self, chunk: (i32, i32), is_start: &dyn Fn((i32, i32)) -> bool) {
        for x in chunk.0 - RADIUS..=chunk.0 + RADIUS {
            for z in chunk.1 - RADIUS..=chunk.1 + RADIUS {
                if self.visited.insert((x, z)) && is_start((x, z)) {
                    self.insert((x, z));
                }
            }
        }
    }

    fn insert(&mut self, start: (i32, i32)) {
        self.buckets = self.buckets.max(FIRST_BUCKETS);
        let wanted = self.list.len() + 1;
        // `_Desired_grow_bucket_count`: eight times as many below 512, else the next power of two.
        if wanted > self.buckets {
            self.buckets = if self.buckets < 512 && self.buckets * 8 >= wanted { self.buckets * 8 } else { wanted.next_power_of_two() };
            let old = std::mem::take(&mut self.list);
            old.into_iter().for_each(|entry| place(&mut self.list, entry, self.buckets));
        }
        place(&mut self.list, (start, hash(start)), self.buckets);
    }

    /// The starts in the order the feature visits them.
    pub fn order(&self) -> Vec<(i32, i32)> {
        self.list.iter().map(|entry| entry.0).collect()
    }
}
