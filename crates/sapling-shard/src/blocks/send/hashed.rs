//! The iteration orders of the Linux BDS's libc++ hash containers keyed by a block or sub-chunk
//! position, hashed by boost's `hash_combine` of x, y and z from 0 (`std::hash<int>` is the
//! identity there). A lit Nether portal's cells come out of an `unordered_set` begun with 64
//! buckets, and the blocks changed together go out a sub-chunk at a time from a fresh
//! `unordered_map` (docs/research/nether-dimension.md, "Section order").

use std::collections::HashMap;

/// boost's `hash_combine` of `x`, `y`, `z`, each sign-extended, from a seed of 0.
fn hash(key: [i32; 3]) -> u64 {
    key.into_iter().fold(0, |seed: u64, part| seed ^ (part as i64 as u64).wrapping_add(0x9e37_79b9).wrapping_add(seed << 6).wrapping_add(seed >> 2))
}

/// libc++'s `__constrain_hash`.
fn bucket(hash: u64, buckets: u64) -> u64 {
    if buckets.is_power_of_two() { hash & (buckets - 1) } else { hash % buckets }
}

fn next_prime(mut n: u64) -> u64 {
    while n < 2 || (2..).take_while(|d| d * d <= n).any(|d| n % d == 0) {
        n += 1;
    }
    n
}

/// A libc++ `unordered_set`'s list: a key whose bucket is empty goes in front, one whose bucket is
/// used just before that bucket's first; growing (past one key a bucket) regroups the list in its
/// order (`__do_rehash`).
pub(crate) struct Ordered {
    keys: Vec<([i32; 3], u64)>,
    buckets: u64,
}

impl Ordered {
    pub(crate) fn with_buckets(buckets: u64) -> Self {
        Self { keys: Vec::new(), buckets }
    }

    /// Adds `key` as `emplace` does; false if it was there.
    pub(crate) fn insert(&mut self, key: [i32; 3]) -> bool {
        if self.keys.iter().any(|(at, _)| *at == key) {
            return false;
        }
        let size = self.keys.len() as u64;
        if self.buckets == 0 || size + 1 > self.buckets {
            let power_of_two = self.buckets > 2 && self.buckets.is_power_of_two();
            self.rehash((2 * self.buckets + u64::from(!power_of_two)).max(size + 1));
        }
        let hashed = hash(key);
        let own = bucket(hashed, self.buckets);
        let at = self.keys.iter().position(|(_, other)| bucket(*other, self.buckets) == own).unwrap_or(0);
        self.keys.insert(at, (key, hashed));
        true
    }

    pub(crate) fn into_order(self) -> Vec<[i32; 3]> {
        self.keys.into_iter().map(|(key, _)| key).collect()
    }

    fn rehash(&mut self, wanted: u64) {
        let buckets = match wanted {
            1 => 2,
            n if n.is_power_of_two() => n,
            n => next_prime(n),
        };
        if buckets <= self.buckets {
            return;
        }
        self.buckets = buckets;
        let mut list: Vec<([i32; 3], u64)> = Vec::with_capacity(self.keys.len());
        let mut firsts: HashMap<u64, usize> = HashMap::new();
        let mut run = None;
        for (key, hashed) in std::mem::take(&mut self.keys) {
            let own = bucket(hashed, buckets);
            if run == Some(own) {
                list.push((key, hashed));
            } else if let Some(&first) = firsts.get(&own) {
                list.insert(first, (key, hashed));
                firsts.values_mut().filter(|at| **at > first).for_each(|at| *at += 1);
            } else {
                firsts.insert(own, list.len());
                list.push((key, hashed));
                run = Some(own);
            }
        }
        self.keys = list;
    }
}

/// `cells` grouped by sub-chunk (its corner), in the order a fresh libc++ map of sub-chunks filled
/// in `cells`' order iterates; each sub-chunk's cells in their order.
pub(crate) fn by_section<T>(cells: impl IntoIterator<Item = ([i32; 3], T)>) -> Vec<([i32; 3], Vec<([i32; 3], T)>)> {
    let mut sections = Ordered::with_buckets(0);
    let mut grouped: HashMap<[i32; 3], Vec<([i32; 3], T)>> = HashMap::new();
    for (pos, value) in cells {
        let section = pos.map(|c| c >> 4);
        sections.insert(section);
        grouped.entry(section).or_default().push((pos, value));
    }
    sections.into_order().into_iter().map(|section| (section.map(|c| c * 16), grouped.remove(&section).unwrap_or_default())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fill_goes_out_by_sub_chunk_as_bds_sent_it() {
        // research/captures/bds-n13sheet.txt: /fill -8 -27 -8 16 -27 8 stone.
        let cells = (-8..=16).flat_map(|x| (-8..=8).map(move |z| ([x, -27, z], ())));
        let corners: Vec<[i32; 3]> = by_section(cells).into_iter().map(|(corner, _)| corner).collect();
        assert_eq!(corners, [[0, -32, 0], [0, -32, -16], [-16, -32, 0], [16, -32, -16], [16, -32, 0], [-16, -32, -16]]);
    }

    #[test]
    fn a_lit_frame_s_cells_come_out_of_a_set_of_64_buckets_as_bds_wrote_them() {
        // research/captures/bds-portal-sections-gdb-hits.txt: the x 60 frame, z 20..40, y -59..-39.
        let mut set = Ordered::with_buckets(64);
        (20..=40).for_each(|z| (-59..=-39).for_each(|y| _ = set.insert([60, y, z])));
        let order = set.into_order();
        assert_eq!(order[..6], [[60, -43, 40], [60, -44, 40], [60, -45, 40], [60, -46, 40], [60, -55, 40], [60, -56, 40]]);
        assert_eq!(order[28..32], [[60, -39, 40], [60, -47, 34], [60, -41, 40], [60, -49, 34]]);
    }
}
