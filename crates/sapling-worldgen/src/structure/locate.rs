//! `/locate structure` for the grid-placed kinds (`StructureFeature::
//! findNearestFeaturePositionBySpacing` 141efb740): rings of grid regions around the origin's,
//! up to 100 out, each ring's boundary scanned x-major; the first start found wins, not the
//! nearest one.

use super::{StructureKind, Structures};
use crate::density::Density;

const MAX_RINGS: i32 = 100;

impl Structures {
    /// The chunk BDS's `/locate` answers from a block position, for kinds placed on a grid.
    pub fn locate(&self, density: &Density, kind: StructureKind, (x, z): (i32, i32)) -> Option<(i32, i32)> {
        let (grid, java) = match (kind.grid(), kind.java_grid()) {
            (Some(grid), _) => (grid, false),
            (None, Some(grid)) => (grid, true),
            (None, None) => return None,
        };
        let spacing = grid.spacing;
        let (cx, cz) = (x >> 4, z >> 4);
        for ring in 0..=MAX_RINGS {
            for dx in -ring..=ring {
                for dz in -ring..=ring {
                    if dx.abs() != ring && dz.abs() != ring {
                        continue;
                    }
                    let region = ((dx * spacing + cx).div_euclid(spacing) * spacing, (dz * spacing + cz).div_euclid(spacing) * spacing);
                    let candidate = if java { grid.java_candidate(self.seed, region) } else { grid.candidate(self.level_seed, region) };
                    if self.is_start(density, kind, candidate) {
                        return Some(candidate);
                    }
                }
            }
        }
        None
    }

    /// The stronghold chunk an eye of ender flies to from a block position
    /// (`StrongholdFeature::getNearestGeneratedFeature` 1483745b0): the spiral's starts, then
    /// 200-chunk cells in rings round the origin's (the first ring the whole 3x3, a cell holding
    /// a spiral start skipped), until a ring ends with a candidate; the nearest by squared chunk
    /// distance wins, the first listed on a tie.
    pub fn nearest_stronghold(&self, density: &Density, (x, z): (i32, i32)) -> Option<(i32, i32)> {
        use super::stronghold::{cell_draws, cell_of};
        let (cx, cz) = (x >> 4, z >> 4);
        let spiral = self.strongholds(density);
        let mut found = spiral.to_vec();
        let (gx, gz) = (cell_of(cx), cell_of(cz));
        let distance = |&(sx, sz): &(i32, i32)| {
            let (dx, dz) = (sx.wrapping_sub(cx), sz.wrapping_sub(cz));
            dz.wrapping_mul(dz).wrapping_add(dx.wrapping_mul(dx))
        };
        for ring in 1..MAX_RINGS {
            for ix in gx - ring..=gx + ring {
                for iz in gz - ring..=gz + ring {
                    if ring >= 2 && (ix - gx).abs() < ring && (iz - gz).abs() < ring {
                        continue;
                    }
                    if spiral.iter().any(|&(sx, sz)| cell_of(sx) == ix && cell_of(sz) == iz) {
                        continue;
                    }
                    // BDS compares the chance as a double here: raw * 2^-32 < 0.25.
                    if let Some((start, _)) = cell_draws(self.level_seed, (ix, iz)).filter(|&(_, chance)| chance < 1 << 30) {
                        found.push(start);
                    }
                }
            }
            if !found.is_empty() {
                return found.iter().copied().min_by_key(distance);
            }
        }
        None
    }
}
