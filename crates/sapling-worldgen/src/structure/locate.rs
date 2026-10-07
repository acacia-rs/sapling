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
}
