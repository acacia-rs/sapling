//! Terrain adjustments around structures, added to the density in `_prepareHeights` (14822eb60)
//! after the noodle caves and before the aquifer. Two systems:
//! - legacy `FeatureTerrainAdjustments` entries per piece (`setBeardifier` 144ccf8f0 for village
//!   pieces, `setBeardAndShaver` 144cd05a0 for ancient city pieces, `setBury` 144cd0700 for
//!   stronghold pieces), summed in f32 by 144cd11f0;
//! - v2 jigsaw structures' rigid pieces and jigsaw junctions (collected by 144cd4450), Java's
//!   `Beardifier` by the structure's `terrain_adaptation`, summed in f64 by
//!   `calculateContribution` 144cd50c0.
//!
//! The density becomes `d1 = f32(density + v2)`, then `d1 + legacy(d1)`; legacy burying only
//! fills where `d1` plus the legacy beards is not solid.

mod collect;
mod kernel;

use super::BoundingBox;

/// v2 `terrain_adaptation`, numbered as BDS stores it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Adaptation {
    Bury = 1,
    BeardThin = 2,
    BeardBox = 3,
    Encapsulate = 4,
}

/// A jigsaw connection's source column and ground height (Java's `JigsawJunction`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Junction {
    pub x: i32,
    pub ground: i32,
    pub z: i32,
}

/// A legacy beard (and shaver) entry: the piece box grown by 2 in x and z, its ground height,
/// and for shavers the middle, half size and the radius margins below and above the ground.
#[derive(Debug, Clone, Copy)]
struct Beard {
    bb: BoundingBox,
    ground: i32,
    middle: (i32, i32),
    radius: f32,
    margins: [f32; 2],
}

impl Beard {
    fn new(piece: BoundingBox, ground_offset: i32, margins: [f32; 2]) -> Self {
        let bb = BoundingBox::new(piece.x0 - 2, piece.y0, piece.z0 - 2, piece.x1 + 2, piece.y1, piece.z1 + 2);
        let (sx, sz) = (piece.x1 - piece.x0 + 5, piece.z1 - piece.z0 + 5);
        let middle = (sx / 2 + bb.x0, sz / 2 + bb.z0);
        Self { bb, ground: piece.y0 + ground_offset, middle, radius: (sx as f32 * 0.5).max(sz as f32 * 0.5), margins }
    }
}

#[derive(Debug, Clone, Copy)]
struct Rigid {
    bb: BoundingBox,
    adaptation: Adaptation,
    ground_delta: i32,
}

/// The adjustments reaching one chunk, with each column's entries.
#[derive(Default)]
pub struct Adjustments {
    shavers: Vec<Beard>,
    beards: Vec<Beard>,
    buries: Vec<BoundingBox>,
    rigids: Vec<Rigid>,
    junctions: Vec<Junction>,
    columns: Vec<[Vec<Entry>; 5]>,
}

/// An entry reaching a column, with the heights outside which it adds exactly zero (skipped; the sums
/// start at +0.0 and never hold -0.0, so adding +0.0 or -0.0 changes nothing).
#[derive(Clone, Copy)]
struct Entry {
    low: i32,
    high: i32,
    index: u32,
}

impl Entry {
    fn at(entries: &[Entry], y: i32) -> impl Iterator<Item = usize> + '_ {
        entries.iter().filter(move |e| e.low <= y && y <= e.high).map(|e| e.index as usize)
    }
}

/// How far outside a box (0 inside) in x and z, and the raw signed distances.
fn outside(bb: &BoundingBox, x: i32, z: i32) -> (i32, i32, i32, i32) {
    let (rx, rz) = ((bb.x0 - x).max(x - bb.x1), (bb.z0 - z).max(z - bb.z1));
    (rx.max(0), rz.max(0), rx, rz)
}

impl Adjustments {
    pub fn is_empty(&self) -> bool {
        self.shavers.is_empty() && self.beards.is_empty() && self.buries.is_empty() && self.rigids.is_empty() && self.junctions.is_empty()
    }

    /// The density at a block of the chunk, from its unadjusted `density`.
    pub fn apply(&self, [x, y, z]: [i32; 3], density: f32) -> f32 {
        let [shavers, beards, buries, rigids, junctions] = &self.columns[((z & 15) << 4 | (x & 15)) as usize];
        let mut v2 = 0.0f64;
        for i in Entry::at(rigids, y) {
            v2 += self.rigid(&self.rigids[i], [x, y, z]);
        }
        let mut joined = 0.0f64;
        for i in Entry::at(junctions, y) {
            let j = self.junctions[i];
            let dy = y - j.ground;
            joined += kernel::v2_beard(x - j.x, dy, z - j.z, dy) as f64 * 0.4;
        }
        let d1 = (v2 + joined + density as f64) as f32;
        let mut legacy = 0.0f32;
        for i in Entry::at(shavers, y) {
            legacy += Self::shaver(&self.shavers[i], [x, y, z]);
        }
        for i in Entry::at(beards, y) {
            let b = &self.beards[i];
            let (dx, dz, _, _) = outside(&b.bb, x, z);
            legacy += kernel::legacy_beard(dx, y - b.ground, dz) * 0.8;
        }
        if d1 + legacy <= 0.0 {
            for i in Entry::at(buries, y) {
                legacy += Self::bury(&self.buries[i], [x, y, z]);
            }
        }
        legacy + d1
    }

    /// The heights where each kind of entry can add anything (the kernels are zero 12 blocks out, the
    /// legacy bury 14 half-steps out): shavers, beards, buries, rigids, junctions.
    fn heights(&self) -> [Vec<(i32, i32)>; 5] {
        let beard = |b: &Beard| (b.ground.min(b.bb.y1) - 12, b.ground.max(b.bb.y1) + 11);
        let rigid = |r: &Rigid| {
            let ground = r.bb.y0 + r.ground_delta;
            match r.adaptation {
                Adaptation::Bury | Adaptation::BeardThin => (ground - 12, ground + 11),
                Adaptation::BeardBox => (ground - 11, r.bb.y1 + 11),
                Adaptation::Encapsulate => (r.bb.y0 - 11, r.bb.y1 + 11),
            }
        };
        [
            self.shavers.iter().map(beard).collect(),
            self.beards.iter().map(|b| (b.ground - 12, b.ground + 11)).collect(),
            self.buries.iter().map(|bb| (bb.y0 - 13, bb.y1.max(bb.y0) + 13)).collect(),
            self.rigids.iter().map(rigid).collect(),
            self.junctions.iter().map(|j| (j.ground - 12, j.ground + 11)).collect(),
        ]
    }

    fn shaver(b: &Beard, [x, y, z]: [i32; 3]) -> f32 {
        let above = y > b.ground;
        let clamped = if y > b.bb.y1 { b.bb.y1 } else if above { y } else { b.ground };
        let (dx, dz, _, _) = outside(&b.bb, x, z);
        let (fx, fy, fz) = ((x - b.middle.0) as f32, (y - clamped) as f32, (z - b.middle.1) as f32);
        let distance = (fx * fx + fy * fy + fz * fz).sqrt();
        let reach = b.margins[usize::from(above)] + b.radius;
        let scale = if 0.0 < reach && distance <= reach { 1.5 - distance / reach } else { 0.0 };
        kernel::legacy_beard(dx, y - clamped, dz) * scale
    }

    /// Legacy burying: full inside the box's columns, fading over six blocks (whole-block steps).
    fn bury(bb: &BoundingBox, [x, y, z]: [i32; 3]) -> f32 {
        let clamped = y.clamp(bb.y0, bb.y1.max(bb.y0));
        let (dx, dz, _, _) = outside(bb, x, z);
        let dy = ((y - clamped) as f32 * 0.5) as i32;
        let distance = ((dz * dz + dx * dx + dy * dy) as f64).sqrt() as i32 as f32 / 6.0;
        if distance > 1.0 {
            0.0
        } else if distance > 0.0 {
            1.0 - distance
        } else {
            1.0
        }
    }

    fn rigid(&self, r: &Rigid, [x, y, z]: [i32; 3]) -> f64 {
        let (dx, dz, rx, rz) = outside(&r.bb, x, z);
        let ground = r.bb.y0 + r.ground_delta;
        let to_ground = y - ground;
        match r.adaptation {
            Adaptation::Bury if rx <= 5 && rz <= 5 => kernel::bury(2 * dx, 2 * dz, to_ground) as f64,
            Adaptation::BeardThin if rx <= 11 && rz <= 11 => kernel::v2_beard(dx, to_ground, dz, to_ground) as f64 * 0.8,
            Adaptation::BeardBox if rx <= 11 && rz <= 11 => {
                let dy = (ground - y).max(y - r.bb.y1);
                if dy > 11 { 0.0 } else { kernel::v2_beard(dx, dy.max(0), dz, to_ground) as f64 * 0.8 }
            }
            Adaptation::Encapsulate if rx <= 11 && rz <= 11 => {
                let dy = (r.bb.y0 - y).max(y - r.bb.y1);
                if dy > 11 { 0.0 } else { kernel::bury(dx, dz, dy.max(0)) as f64 * 0.8 }
            }
            _ => 0.0,
        }
    }

    /// Indexes each column's entries that can reach it.
    fn finish(&mut self, (cx, cz): (i32, i32)) {
        let near = |bb: &BoundingBox, reach: i32, x: i32, z: i32| {
            let (dx, dz, _, _) = outside(bb, x, z);
            dx <= reach && dz <= reach
        };
        let heights = self.heights();
        self.columns = (0..256)
            .map(|i| {
                let (x, z) = (cx * 16 + (i & 15), cz * 16 + (i >> 4));
                let pick = |kind: usize, hits: &mut dyn Iterator<Item = bool>| {
                    let entry = |(i, _): (usize, bool)| Entry { low: heights[kind][i].0, high: heights[kind][i].1, index: i as u32 };
                    hits.enumerate().filter(|(_, hit)| *hit).map(entry).collect()
                };
                [
                    pick(0, &mut self.shavers.iter().map(|b| near(&b.bb, 11, x, z))),
                    pick(1, &mut self.beards.iter().map(|b| near(&b.bb, 11, x, z))),
                    pick(2, &mut self.buries.iter().map(|b| near(b, 6, x, z))),
                    pick(3, &mut self.rigids.iter().map(|r| near(&r.bb, 11, x, z))),
                    pick(4, &mut self.junctions.iter().map(|j| (-11..=12).contains(&(j.x - x)) && (-11..=12).contains(&(j.z - z)))),
                ]
            })
            .collect();
    }
}
