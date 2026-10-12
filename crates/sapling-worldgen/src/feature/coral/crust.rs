//! `CoralCrustFeature::place` (148288070): dead coral and stone templates (`coralcrust/crust1..5`, a
//! cluster of 4 to 8 tries, or one `outcropping1..6`) placed as `LegacyStructureTemplate::placeInWorld`
//! does, then `_placeCoralBase` (1482895b0) grows coral from each template's data markers.

use std::sync::atomic::{AtomicUsize, Ordering};

use super::{CORAL_BLOCKS, OPPOSITE, Reef, block, coral_index, fits, is_water, solid, step, up};

/// The templates' `coral_block[variant=dead_coral]` as a `CoralColor`. BDS's legacy converter (141f71f20) picks it
/// per template when the template loads, `Random(steady clock in ms).nextInt(5)` at 141f73173: no seed decides it.
/// Bubble is what the stock oracle worlds happened to get.
static DEAD_CORAL: AtomicUsize = AtomicUsize::new(2);

/// Process-wide, for replaying a BDS run whose crusts took another dead coral.
pub fn set_dead_coral(color: usize) {
    DEAD_CORAL.store(color.min(4), Ordering::Relaxed);
}
use crate::density::SEA_LEVEL;
use crate::feature::data::coral_crust::{CRUSTS, Crust, KINDS};
use crate::feature::view::View;
use crate::random::Mt19937;
use crate::structure::template::{Rotation, marker_hash};

/// Where a crust of `size` turned `rotation` is probed from its origin (0x14a925dc0..0x14a925e10);
/// `placed` is the way back (0x14a925e14..0x14a925e64).
fn probe(size: i32, rotation: i32) -> [i32; 2] {
    let r = rotation as usize;
    match size {
        0 => [[6, 6], [-7, 6], [-7, -7], [6, -7]][r],
        1 => [[6, 6], [-6, 6], [-7, -7], [6, -7]][r],
        4 => [[3, 6], [-7, 3], [-2, -7], [7, -3]][r],
        _ => [[4, 4], [-4, 4], [-4, -4], [4, -4]][r],
    }
}

fn placed(size: i32, rotation: i32) -> [i32; 2] {
    let [x, z] = probe(size, rotation);
    [-x, -z]
}

/// `_getCropOffsetFromRot` (148289370): an outcropping's origin from the probe.
fn outcrop_offset(rotation: i32, kind: usize) -> [i32; 3] {
    const TABLE: [[[i32; 3]; 4]; 6] = [
        [[-5, 0, -2], [3, 0, -5], [5, 0, 2], [-2, 0, 5]],
        [[-7, -2, -3], [3, -2, -6], [6, -2, 3], [-2, -2, 5]],
        [[-4, 0, -3], [3, 0, -4], [4, 0, 3], [-3, 0, 4]],
        [[-3, 0, -3], [3, 0, -3], [3, 0, 3], [-3, 0, 3]],
        [[-2, 0, -2], [2, 0, -2], [2, 0, 2], [-2, 0, 2]],
        [[-5, 0, -5], [6, 0, -6], [5, 0, 5], [-5, 0, 5]],
    ];
    TABLE[kind][rotation as usize]
}

pub fn crust(view: &mut View, random: &mut Mt19937, at: [i32; 3]) -> bool {
    let (size, rotation) = (random.next_int(5), random.next_int(4));
    let [px, pz] = probe(size, rotation);
    let mut p = [at[0] + px, at[1], at[2] + pz];
    while p[1] < SEA_LEVEL - 9 && !is_water(block(view, p)) {
        p[1] += 1;
    }
    let below = block(view, up(p, -1));
    if p[1] >= SEA_LEVEL - 9 || !solid(view, up(p, -1)) || coral_index(below).is_some_and(|i| i < 5) {
        return false;
    }
    let [dx, dz] = placed(size, rotation);
    let mut crusts = vec![(size, rotation, [p[0] + dx, p[1], p[2] + dz])];
    if random.next_int(4) != 0 {
        for _ in 0..random.next_int(5) + 4 {
            let (size, rotation) = (random.next_int(5), random.next_int(4));
            let mut offset = || {
                let a = random.next_int(9);
                if random.next_int(2) == 0 { -3 - a } else { a + 3 }
            };
            let (ox, oz) = (offset(), offset());
            let [px, pz] = probe(size, rotation);
            let q = [ox + p[0] + px, p[1], oz + p[2] + pz];
            if solid(view, up(q, -1)) {
                let [dx, dz] = placed(size, rotation);
                crusts.push((size, rotation, [q[0] + dx, q[1], q[2] + dz]));
            }
        }
    }
    let mut reef = Reef::new(view, random);
    if crusts.len() == 1 {
        let kind = reef.random.next_int(6) as usize;
        let [ox, oy, oz] = outcrop_offset(rotation, kind);
        reef.place_crust(&CRUSTS[5 + kind], [p[0] + ox, p[1] + oy, p[2] + oz], rotation);
    } else {
        for (size, rotation, origin) in crusts {
            reef.place_crust(&CRUSTS[size as usize], origin, rotation);
        }
    }
    true
}

impl Reef<'_, '_> {
    fn place_crust(&mut self, crust: &Crust, origin: [i32; 3], rotation: i32) {
        let turn = |(x, y, z): (u8, u8, u8)| {
            let (dx, dy, dz) = Rotation::from_index(rotation).apply((x as i32, y as i32, z as i32));
            [origin[0] + dx, origin[1] + dy, origin[2] + dz]
        };
        for &(x, y, z, kind) in crust.blocks {
            let block = if kind == 0 { CORAL_BLOCKS[5 + DEAD_CORAL.load(Ordering::Relaxed)] } else { KINDS[kind as usize] };
            self.set(turn((x, y, z)), block);
        }
        let markers: Vec<[i32; 3]> = crust.markers.iter().map(|&m| turn(m)).collect();
        self.coral_base(&msvc_order(&markers));
        self.decorate();
    }

    /// `_placeCoralBase`: from each marker on solid ground, a coral run (3 in 4) or a cross and stem.
    fn coral_base(&mut self, markers: &[[i32; 3]]) {
        let color = self.random.next_int(5) as usize;
        let coral = CORAL_BLOCKS[color];
        'markers: for &marker in markers {
            if !solid(self.view, up(marker, -1)) {
                continue;
            }
            let mut m = if solid(self.view, marker) { up(marker, 1) } else { marker };
            if self.random.next_int(4) == 0 {
                if !fits(self.view, m) {
                    continue;
                }
                for face in [5, 2, 4, 3] {
                    let n = step(m, face);
                    if coral_index(block(self.view, n)).is_none() {
                        self.set(n, coral);
                    }
                }
                self.set(m, coral);
                if self.set(up(m, 1), coral) && self.random.next_int(4) == 0 {
                    self.tops.push(up(m, 2));
                }
                continue;
            }
            let length = self.random.next_int(3);
            if !is_water(block(self.view, m)) {
                continue;
            }
            self.set(m, coral);
            let mut along_z = self.random.next_int(2) != 0;
            for _ in 0..=length {
                // The face recorded points back to where the run came from.
                let face = match (along_z, self.random.next_int(2) == 0) {
                    (false, true) => 4,
                    (false, false) => 5,
                    (true, true) => 3,
                    (true, false) => 2,
                };
                m = step(m, OPPOSITE[face]);
                if !fits(self.view, m) {
                    break;
                }
                if coral_index(block(self.view, m)).is_some() {
                    continue 'markers;
                }
                if !self.set(m, coral) {
                    break;
                }
                if self.random.next_int(4) == 0 {
                    self.tops.push(up(m, 1));
                }
                for bound in [2, 2, 3] {
                    if self.random.next_int(bound) == 0 {
                        self.sides.push((m, face));
                    }
                }
                along_z = !along_z;
            }
        }
    }
}

/// Keys in the iteration order of an MSVC `std::unordered_map` they were inserted into in order: a
/// key goes in front of its bucket's first node, or to the list's end; 8 buckets at load factor 1,
/// growing x8 below 512 (x2 above), a rehash re-placing the list in order.
fn msvc_order(keys: &[[i32; 3]]) -> Vec<[i32; 3]> {
    fn place(list: &mut Vec<([i32; 3], u64)>, key: [i32; 3], hash: u64, buckets: u64) {
        let bucket = hash & (buckets - 1);
        match list.iter().position(|&(_, h)| h & (buckets - 1) == bucket) {
            Some(first) => list.insert(first, (key, hash)),
            None => list.push((key, hash)),
        }
    }
    let (mut list, mut buckets) = (Vec::new(), 8u64);
    for &key in keys {
        if list.len() as u64 + 1 > buckets {
            buckets *= if buckets < 512 { 8 } else { 2 };
            for (k, h) in std::mem::take(&mut list) {
                place(&mut list, k, h, buckets);
            }
        }
        place(&mut list, key, marker_hash((key[0], key[1], key[2])), buckets);
    }
    list.into_iter().map(|(k, _)| k).collect()
}
