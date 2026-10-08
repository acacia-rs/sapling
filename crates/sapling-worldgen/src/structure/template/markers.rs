//! Template data markers in the order BDS visits them: `LegacyStructureTemplate::getMarkers`
//! (141ee8850) fills an MSVC `std::unordered_map<BlockPos, std::string>`, iterated in its list
//! order, so markers whose draws depend on order (shipwreck chests) follow the map's buckets.

use super::{Rotation, Template};

/// BDS's `std::hash<BlockPos>`: FNV-1a of each coordinate's bytes, boost-combined x, y, z.
fn hash((x, y, z): (i32, i32, i32)) -> u64 {
    let fnv = |v: i32| v.to_le_bytes().iter().fold(0xcbf2_9ce4_8422_2325_u64, |h, &b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3));
    let combine = |seed: u64, v: u64| v.wrapping_add(0x9e37_79b9).wrapping_add(seed >> 2).wrapping_add(seed << 6) ^ seed;
    let h = fnv(x).wrapping_add(0x9e37_79b9);
    let h = combine(h, fnv(y));
    combine(h, fnv(z))
}

/// Keys in an MSVC unordered map's iteration order after inserting them in `keys`' order: a key
/// whose bucket is empty goes to the list's end, else after its bucket's last element. Holds
/// while there are no more keys than the initial eight buckets (no rehash).
fn msvc_order(keys: &[(i32, i32, i32)]) -> Vec<usize> {
    debug_assert!(keys.len() <= 8, "MSVC rehash not modelled");
    let mut list: Vec<(usize, u64)> = Vec::new();
    for (i, &key) in keys.iter().enumerate() {
        let bucket = hash(key) & 7;
        match list.iter().rposition(|&(_, b)| b == bucket) {
            Some(last) => list.insert(last + 1, (i, bucket)),
            None => list.push((i, bucket)),
        }
    }
    list.into_iter().map(|(i, _)| i).collect()
}

impl Template {
    /// Data markers placed at `origin` turned by `rotation`: world position and metadata.
    pub fn markers_at(&self, origin: (i32, i32, i32), rotation: Rotation) -> Vec<((i32, i32, i32), &str)> {
        let placed: Vec<((i32, i32, i32), &str)> = self
            .markers
            .iter()
            .map(|(index, data)| {
                let (dx, dy, dz) = rotation.apply(self.blocks[*index].pos);
                ((origin.0 + dx, origin.1 + dy, origin.2 + dz), data.as_str())
            })
            .collect();
        let positions: Vec<_> = placed.iter().map(|(p, _)| *p).collect();
        msvc_order(&positions).into_iter().map(|i| placed[i]).collect()
    }
}
