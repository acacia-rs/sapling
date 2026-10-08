//! Overworld structures (docs/research/worldgen-bds-structures.md): where each one starts, by
//! BDS's legacy `StructureFeature` checks (`isFeatureChunk`, vtable slot 5) and the v2 jigsaw
//! structure sets.

mod ancient_city;
mod area;
mod bury;
mod canvas;
mod data_driven;
mod igloo;
mod kind;
mod jigsaw;
mod locate;
mod mansion;
mod mineshaft;
mod monument;
mod nearby;
mod ocean_ruin;
mod outpost;
mod piece;
mod placement;
mod probe;
mod ruined_portal;
mod ruined_portal_age;
mod scattered;
mod shipwreck;
mod stage;
mod stronghold;
pub(crate) mod template;
mod templated;
#[cfg(test)]
mod tests;
mod treasure;
mod village;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::density::Density;
pub use area::BoundingBox;
pub use bury::bury;
pub use kind::StructureKind;
use kind::*;
use probe::Probe;
pub use stage::rejoin_connections;

const SEA_LEVEL: i32 = 63;

/// Structure starts for one world seed, and the pieces of those already generated.
pub struct Structures {
    seed: i64,
    level_seed: u32,
    mineshaft_seed: placement::MineshaftSeed,
    /// Mineshaft chances already drawn: every column asks for the 15x15 chunks around it.
    mineshaft_chunks: Mutex<HashMap<(i32, i32), bool>>,
    /// Grid candidates' start checks, which every chunk around them repeats.
    grid_starts: Mutex<HashMap<(StructureKind, (i32, i32)), bool>>,
    strongholds: OnceLock<Vec<(i32, i32)>>,
    mineshafts: Mutex<HashMap<(i32, i32), Arc<mineshaft::Mineshaft>>>,
    monuments: Mutex<HashMap<(i32, i32), Arc<monument::Monument>>>,
    stronghold_pieces: Mutex<HashMap<(i32, i32), Arc<stronghold::Stronghold>>>,
    templates: templated::TemplateStarts,
}

impl Structures {
    pub fn new(seed: i64) -> Self {
        let level_seed = seed as u32;
        let mineshaft_seed = placement::MineshaftSeed::new(level_seed);
        Self {
            seed,
            level_seed,
            mineshaft_seed,
            mineshaft_chunks: Mutex::new(HashMap::new()),
            grid_starts: Mutex::new(HashMap::new()),
            strongholds: OnceLock::new(),
            mineshafts: Mutex::new(HashMap::new()),
            monuments: Mutex::new(HashMap::new()),
            stronghold_pieces: Mutex::new(HashMap::new()),
            templates: Default::default(),
        }
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
            StructureKind::Mineshaft => *self.mineshaft_chunks.lock().unwrap().entry(chunk).or_insert_with(|| self.mineshaft_seed.is_start(chunk)),
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
            StructureKind::TrialChambers => {
                let (x, y, z) = self.v2_position(density, kind, chunk);
                !DEEP_DARK.contains(&probe.biome(x, y, z))
            }
            StructureKind::TrailRuins => {
                let (x, y, z) = self.v2_position(density, kind, chunk);
                TRAIL_RUINS_BIOMES.contains(&probe.biome(x, y, z))
            }
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
