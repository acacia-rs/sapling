//! Overworld structures (docs/research/worldgen-bds-structures.md): where each one starts, by
//! BDS's legacy `StructureFeature` checks (`isFeatureChunk`, vtable slot 5) and the v2 jigsaw
//! structure sets.

mod area;
mod canvas;
mod kind;
mod locate;
mod mineshaft;
mod nearby;
mod piece;
mod placement;
mod probe;
mod stage;
mod stronghold;
#[cfg(test)]
mod tests;
mod treasure;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::density::Density;
pub use area::BoundingBox;
pub use kind::StructureKind;
use kind::*;
use probe::Probe;
pub use stage::rejoin_fences;

const SEA_LEVEL: i32 = 63;

/// Structure starts for one world seed, and the pieces of those already generated.
pub struct Structures {
    seed: i64,
    level_seed: u32,
    strongholds: OnceLock<Vec<(i32, i32)>>,
    mineshafts: Mutex<HashMap<(i32, i32), Arc<mineshaft::Mineshaft>>>,
}

impl Structures {
    pub fn new(seed: i64) -> Self {
        Self { seed, level_seed: seed as u32, strongholds: OnceLock::new(), mineshafts: Mutex::new(HashMap::new()) }
    }

    /// Whether `kind` starts in `chunk` (`isFeatureChunk`; for v2 sets the grid and a biome).
    pub fn is_start(&self, density: &Density, kind: StructureKind, chunk: (i32, i32)) -> bool {
        let probe = Probe { density };
        let seed = self.level_seed;
        let centre = |y: i32| (chunk.0 * 16 + 8, y, chunk.1 * 16 + 8);
        if let Some(grid) = kind.grid()
            && grid.candidate(seed, chunk) != chunk
        {
            return false;
        }
        if let Some(grid) = kind.java_grid()
            && grid.java_candidate(self.seed, chunk) != chunk
        {
            return false;
        }
        match kind {
            StructureKind::Village => probe.contains_only(centre(probe.surface(chunk)), 2, VILLAGE_BIOMES),
            StructureKind::Stronghold => self.strongholds(density).contains(&chunk) || stronghold::grid_start(seed, chunk) == Some(chunk),
            StructureKind::Temple => {
                let (x, y, z) = centre(probe.surface(chunk));
                TEMPLE_BIOMES.contains(&probe.biome(x, y, z)) && probe.min_height(chunk) >= SEA_LEVEL
            }
            StructureKind::Mineshaft => placement::is_mineshaft_chunk(seed, chunk),
            StructureKind::Mansion => probe.contains_only(centre(probe.surface(chunk)), 32, MANSION_BIOMES),
            StructureKind::BuriedTreasure => probe.contains_only(centre(probe.surface(chunk)), 3, BURIED_TREASURE_BIOMES),
            StructureKind::Outpost => probe.contains_only(centre(probe.surface(chunk)), 0, OUTPOST_BIOMES),
            StructureKind::Monument => self.monument_biomes(&probe, chunk),
            StructureKind::OceanRuins => {
                probe.contains_only(centre(probe.surface(chunk)), 0, OCEAN_RUINS_BIOMES) && !self.near_monument(&probe, chunk)
            }
            StructureKind::Shipwreck => self.shipwreck_biomes(&probe, chunk) && !self.near_monument(&probe, chunk),
            StructureKind::RuinedPortal => true,
            StructureKind::AncientCity => probe.contains_only((chunk.0 * 16, -27, chunk.1 * 16), 0, DEEP_DARK),
            // TODO: BDS checks the biome at the jigsaw start piece's centre (14196cc20); the chunk centre approximates it.
            StructureKind::TrialChambers => !DEEP_DARK.contains(&probe.biome(chunk.0 * 16 + 8, -30, chunk.1 * 16 + 8)),
            StructureKind::TrailRuins => TRAIL_RUINS_BIOMES.contains(&probe.biome(chunk.0 * 16 + 8, probe.surface(chunk) - 15, chunk.1 * 16 + 8)),
        }
    }

    fn is_monument(&self, probe: &Probe, chunk: (i32, i32)) -> bool {
        MONUMENT.candidate(self.level_seed, chunk) == chunk && self.monument_biomes(probe, chunk)
    }

    fn monument_biomes(&self, probe: &Probe, chunk: (i32, i32)) -> bool {
        let centre = (chunk.0 * 16 + 8, probe.surface(chunk), chunk.1 * 16 + 8);
        probe.contains_only(centre, 16, DEEP_OCEANS) && probe.contains_only(centre, 29, MONUMENT_SURROUNDINGS)
    }

    /// A monument start in the 10x10 chunks from (x-5, z-5) to (x+4, z+4).
    fn near_monument(&self, probe: &Probe, (x, z): (i32, i32)) -> bool {
        (x - 5..=x + 4).any(|mx| (z - 5..=z + 4).any(|mz| self.is_monument(probe, (mx, mz))))
    }

    /// The centre's biome a shipwreck biome, and every block within 20 (10 on beaches) the same.
    fn shipwreck_biomes(&self, probe: &Probe, chunk: (i32, i32)) -> bool {
        let centre = (chunk.0 * 16 + 8, probe.surface(chunk), chunk.1 * 16 + 8);
        let biome = probe.biome(centre.0, centre.1, centre.2);
        let radius = if BEACHES.contains(&biome) { 10 } else { 20 };
        probe.contains_only(centre, 0, SHIPWRECK_BIOMES) && probe.contains_only(centre, radius, &[biome])
    }

    fn strongholds(&self, density: &Density) -> &[(i32, i32)] {
        self.strongholds.get_or_init(|| stronghold::spiral(self.level_seed, &|chunk| self.is_start(density, StructureKind::Village, chunk)))
    }
}
