//! Terrain burying stronghold pieces: `createStructureStart` registers every piece box with
//! `FeatureTerrainAdjustments::setBury` (144d065e0), and `_prepareHeights` (14822eb60) adds, after
//! the noodle caves and before the aquifer, `1 - |(dx, dy/2, dz)|/6` clamped to 0..1 per box (dx, dz
//! outside the box, dy from its floor), read from a f32 table (144cd3d40, summed in 144cd50c0).

use super::stronghold::Stronghold;
use super::{BoundingBox, StructureKind, Structures};
use crate::density::Density;

/// Pieces bury terrain up to 5 blocks beside them, one chunk at most past their stronghold's box.
const REACH_CHUNKS: i32 = 10;

impl Structures {
    /// The stronghold piece boxes whose burying reaches the chunk.
    pub fn bury_boxes(&self, density: &Density, (cx, cz): (i32, i32)) -> Vec<BoundingBox> {
        let column = BoundingBox::new(cx * 16 - 5, -64, cz * 16 - 5, cx * 16 + 20, 319, cz * 16 + 20);
        let starts = self.starts_near(density, StructureKind::Stronghold, (cx, cz), REACH_CHUNKS);
        let strongholds: Vec<std::sync::Arc<Stronghold>> = starts.into_iter().map(|start| self.stronghold(start)).collect();
        strongholds.iter().flat_map(|s| s.pieces.iter().map(|p| p.frame.bb)).filter(|bb| bb.intersects(&column)).collect()
    }
}

/// The burying at a block, in f64 as BDS sums it.
pub fn bury(boxes: &[BoundingBox], [x, y, z]: [i32; 3]) -> f64 {
    let mut sum = 0.0;
    for b in boxes {
        let dx = (b.x0 - x).max(x - b.x1).max(0);
        let dz = (b.z0 - z).max(z - b.z1).max(0);
        let dy = y - b.y0;
        if dx > 5 || dz > 5 || !(-12..12).contains(&dy) {
            continue;
        }
        let half_dy = dy as f64 * 0.5;
        let distance = ((dz * dz) as f64 + half_dy * half_dy + (dx * dx) as f64).sqrt() * 0.166_666_666_666_666_66;
        sum += (1.0 - distance).clamp(0.0, 1.0) as f32 as f64;
    }
    sum
}
