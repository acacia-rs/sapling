//! Starts within a square of chunks, found per grid region where a kind has a grid so that only
//! candidates pay for the biome checks.

use super::{BoundingBox, StructureKind, Structures, stronghold};
use crate::density::Density;

impl Structures {
    /// Every start of `kind` in the chunks within `radius` of `centre`.
    pub fn starts_near(&self, density: &Density, kind: StructureKind, centre: (i32, i32), radius: i32) -> Vec<(i32, i32)> {
        let (x0, z0, x1, z1) = (centre.0 - radius, centre.1 - radius, centre.0 + radius, centre.1 + radius);
        let inside = |(x, z): (i32, i32)| (x0..=x1).contains(&x) && (z0..=z1).contains(&z);
        let grid = kind.grid().map(|g| (g, false)).or(kind.java_grid().map(|g| (g, true)));
        let candidates: Vec<(i32, i32)> = match (kind, grid) {
            (_, Some((grid, java))) => {
                let s = grid.spacing;
                let regions = |a: i32, b: i32| a.div_euclid(s)..=b.div_euclid(s);
                regions(x0, x1)
                    .flat_map(|rx| regions(z0, z1).map(move |rz| (rx * s, rz * s)))
                    .map(|region| if java { grid.java_candidate(self.seed, region) } else { grid.candidate(self.level_seed, region) })
                    .collect()
            }
            (StructureKind::Stronghold, _) => {
                let cells = |a: i32, b: i32| (a as f32 / 200.0).floor() as i32..=(b as f32 / 200.0).floor() as i32;
                let grid = cells(x0, x1)
                    .flat_map(|gx| cells(z0, z1).map(move |gz| (gx * 200 + 100, gz * 200 + 100)))
                    .filter_map(|cell| stronghold::grid_start(self.level_seed, cell));
                self.strongholds(density).iter().copied().chain(grid).collect()
            }
            _ => (x0..=x1).flat_map(|x| (z0..=z1).map(move |z| (x, z))).collect(),
        };
        candidates.into_iter().filter(|&c| inside(c) && self.is_start(density, kind, c)).collect()
    }

    /// Every structure whose rough area reaches the chunk, with its start chunk.
    pub fn areas_touching(&self, density: &Density, chunk: (i32, i32)) -> Vec<(StructureKind, (i32, i32), BoundingBox)> {
        let column = BoundingBox::chunk(chunk.0, chunk.1);
        let mut found = Vec::new();
        for kind in StructureKind::ALL {
            for start in self.starts_near(density, kind, chunk, kind.reach()) {
                let area = kind.rough_area(start);
                if area.intersects(&column) {
                    found.push((kind, start, area));
                }
            }
        }
        found
    }
}
