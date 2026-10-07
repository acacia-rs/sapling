//! The carver stage measured on its own: cells our terrain shape has solid, split by whether BDS
//! emptied them and whether our carvers did. A cell BDS emptied near a decoration block (geode
//! interiors, lush-cave pools) is counted apart: features, not carvers, emptied it.

use std::collections::BTreeMap;

use acacia_world::Chunk;
use sapling_leveldb::LevelDbWorld;
use sapling_worldgen::CellKind;

use super::{BdsCell, TERRAIN_BLOCKS, bds_cell};

/// Geode interiors reach about this far from their shell.
const FEATURE_RADIUS: i32 = 4;

#[derive(Default)]
pub struct CarverTally {
    /// Both emptied it, to the same kind.
    matched: u64,
    /// Both emptied it, to different kinds (air against water, ...).
    wrong_fluid: u64,
    /// We carved, BDS kept it solid.
    extra: u64,
    /// We carved, BDS has a feature block there (placed after carving).
    decorated: u64,
    /// BDS emptied it next to a feature block, we did not.
    by_features: u64,
    /// BDS emptied it right above or below one of our noise caves: a terrain-shape boundary.
    shape_edge: u64,
    /// BDS emptied it, we did not, and no feature explains it.
    missed: u64,
    missed_by_band: BTreeMap<i32, u64>,
    extra_by_band: BTreeMap<i32, u64>,
    first_extra: Option<(i32, i32, i32)>,
    first_missed: Option<(i32, i32, i32)>,
}

impl CarverTally {
    /// Unstable cells are skipped by the caller.
    pub fn add(&mut self, ours: Ours, bds: &BdsCell, at: (i32, i32, i32), world: &LevelDbWorld, chunk: &Chunk) {
        let Ours { before, after, next_to_cave, iceberg } = ours;
        if before != CellKind::Solid {
            return;
        }
        let band = at.1.div_euclid(32) * 32;
        let theirs = bds.kind();
        match (after != CellKind::Solid, theirs != CellKind::Solid) {
            (false, true) if next_to_cave => self.shape_edge += 1,
            (true, false) if !TERRAIN_BLOCKS.contains(&bds.block.as_str()) => self.decorated += 1,
            (true, true) if after == theirs => self.matched += 1,
            (true, true) => self.wrong_fluid += 1,
            (true, false) => {
                self.extra += 1;
                *self.extra_by_band.entry(band).or_default() += 1;
                self.first_extra.get_or_insert(at);
            }
            (false, true) if iceberg || near_feature(world, chunk, at) => self.by_features += 1,
            (false, true) => {
                self.missed += 1;
                *self.missed_by_band.entry(band).or_default() += 1;
                self.first_missed.get_or_insert(at);
            }
            (false, false) => {}
        }
    }

    pub fn print(&self) {
        let carved = self.matched + self.wrong_fluid + self.extra;
        println!("  carver cells BDS decorated afterwards (not counted): {}", self.decorated);
        let ours_to_find = self.matched + self.wrong_fluid + self.missed;
        let percent = |n: u64, of: u64| 100.0 * n as f64 / of.max(1) as f64;
        println!(
            "  carvers: {carved} cells carved; precision {:.3}%, recall {:.3}% ({:.3}% counting feature-emptied and \
             noise-cave-edge cells) (matched {}, wrong fluid {}, extra {}, missed {}, emptied by features {}, noise-cave edge {})",
            percent(self.matched, carved),
            percent(self.matched, ours_to_find),
            percent(self.matched, ours_to_find + self.by_features + self.shape_edge),
            self.matched,
            self.wrong_fluid,
            self.extra,
            self.missed,
            self.by_features,
            self.shape_edge
        );
        let bands = |m: &BTreeMap<i32, u64>| m.iter().map(|(y, n)| format!("{y}:{n}")).collect::<Vec<_>>().join(" ");
        println!("      extra by y/32 {}  first {:?}", bands(&self.extra_by_band), self.first_extra);
        println!("      missed by y/32 {}  first {:?}", bands(&self.missed_by_band), self.first_missed);
    }
}

/// One of our cells: before and after carving, and whether the uncarved shape is empty right
/// above or below it.
pub struct Ours {
    pub before: CellKind,
    pub after: CellKind,
    pub next_to_cave: bool,
    /// Packed ice or snow, which the iceberg feature also cuts out (see judge.rs).
    pub iceberg: bool,
}

/// Any non-terrain solid block within `FEATURE_RADIUS`, looking inside this chunk only.
pub fn near_feature(world: &LevelDbWorld, chunk: &Chunk, (x, y, z): (i32, i32, i32)) -> bool {
    let (x0, z0) = (x.div_euclid(16) * 16, z.div_euclid(16) * 16);
    let r = FEATURE_RADIUS;
    for nx in (x - r).max(x0)..=(x + r).min(x0 + 15) {
        for nz in (z - r).max(z0)..=(z + r).min(z0 + 15) {
            for ny in y - r..=y + r {
                let cell = bds_cell(world, chunk, nx, ny, nz);
                if cell.kind() == CellKind::Solid && !TERRAIN_BLOCKS.contains(&cell.block.as_str()) {
                    return true;
                }
            }
        }
    }
    false
}
