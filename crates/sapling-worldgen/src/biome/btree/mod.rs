//! The overworld multi-noise parameter list as Java's search tree, in cubiomes' encoding, and
//! Java's (and BDS's `1480616f0`) nearest-leaf search with a warm start. Ported from cubiomes
//! `biomenoise.c` (`get_np_dist`, `get_resulting_node`), MIT, Copyright (c) 2020 Cubitect.

mod nodes;
mod params;

use nodes::NODES;
use params::{ORDER, PARAMS, STEPS};

/// Temperature, humidity, continentalness, erosion, depth, weirdness, quantized x10000.
pub type ClimatePoint = [i64; 6];

/// A leaf of the tree; carry it to the next search of the same column as its warm start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Leaf(usize);

impl Leaf {
    /// cubiomes' (Java-era numeric) biome id.
    pub fn biome(self) -> u8 {
        ((NODES[self.0] >> 48) & 0xFF) as u8
    }
}

fn distance(point: &ClimatePoint, index: usize) -> u64 {
    let node = NODES[index];
    let mut sum = 0u64;
    for (i, &value) in point.iter().enumerate() {
        let [min, max] = PARAMS[((node >> (8 * i)) & 0xFF) as usize];
        let above = value - max;
        let below = min - value;
        let d = if above > 0 { above } else if below > 0 { below } else { 0 } as u64;
        sum = sum.wrapping_add(d.wrapping_mul(d));
    }
    sum
}

fn search(point: &ClimatePoint, index: usize, mut best: usize, mut best_distance: u64, mut depth: usize) -> usize {
    if STEPS[depth] == 0 {
        return index;
    }
    let mut step;
    loop {
        step = STEPS[depth];
        depth += 1;
        if index + step < NODES.len() {
            break;
        }
    }
    let mut child = (NODES[index] >> 48) as u16 as usize;
    for _ in 0..ORDER {
        let child_distance = distance(point, child);
        if child_distance < best_distance {
            let found = search(point, child, best, best_distance, depth);
            let found_distance = if found == child { child_distance } else { distance(point, found) };
            if found_distance < best_distance {
                best_distance = found_distance;
                best = found;
            }
        }
        child += step;
        if child >= NODES.len() {
            break;
        }
    }
    best
}

/// The nearest leaf; ties keep `warm` (the previous result), as BDS does within a column.
pub fn nearest(point: &ClimatePoint, warm: Option<Leaf>) -> Leaf {
    let found = match warm {
        Some(Leaf(index)) => search(point, 0, index, distance(point, index), 0),
        None => search(point, 0, 0, u64::MAX, 0),
    };
    Leaf(found)
}
