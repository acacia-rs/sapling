//! Overworld structures (docs/research/worldgen-bds-structures.md): where each one starts, by
//! BDS's legacy `StructureFeature` checks (`isFeatureChunk`, vtable slot 5) and the v2 jigsaw
//! structure sets.

mod adjust;
mod ancient_city;
mod area;
pub(crate) mod bastion;
mod canvas;
mod data_driven;
pub(crate) mod end;
mod end_city;
mod fortress;
mod igloo;
mod kind;
mod jigsaw;
mod locate;
mod mansion;
mod mineshaft;
mod monument;
mod nearby;
pub(crate) mod nether;
mod ocean_ruin;
mod outpost;
mod piece;
mod placement;
mod probe;
mod reaching;
mod ruined_portal;
mod ruined_portal_age;
mod scattered;
mod shipwreck;
mod stage;
mod start_cache;
mod start_map;
mod stronghold;
pub(crate) mod template;
mod templated;
#[cfg(test)]
mod tests;
#[doc(hidden)]
pub mod trace;
mod treasure;
mod village;

use std::sync::OnceLock;

use start_cache::StartCache;
use crate::density::Density;
pub use adjust::Adjustments;
pub use area::BoundingBox;
pub use kind::StructureKind;
#[doc(hidden)]
pub use mineshaft::probe as mineshaft_probe;
use kind::*;
use probe::Probe;
pub use canvas::{Neighbours, Past, Settled, Spawners};
pub use reaching::Reaching;
pub use stage::{Placed, rejoin_connections};
pub use start_map::StartMap;

const SEA_LEVEL: i32 = 63;
/// Per area generated at once: start checks kept (a column asks the 15x15 chunks around it about
/// mineshafts), and generated starts of each kind. A start is a pure function of the seed and its
/// chunk, so one dropped is only built again (docs/research/worldgen-perf.md, "Structure starts").
const CHECKS_LIMIT: usize = 4096;
const STARTS_LIMIT: usize = 256;

/// Structure starts for one world seed, and the pieces of those generated lately.
pub struct Structures {
    seed: i64,
    level_seed: u32,
    mineshaft_seed: placement::MineshaftSeed,
    mineshaft_chunks: StartCache<(i32, i32), bool>,
    /// Grid candidates' start checks, which every chunk around them repeats.
    grid_starts: StartCache<(StructureKind, (i32, i32)), bool>,
    strongholds: OnceLock<Vec<(i32, i32)>>,
    mineshafts: StartCache<(i32, i32), mineshaft::Mineshaft>,
    monuments: StartCache<(i32, i32), monument::Monument>,
    stronghold_pieces: StartCache<(i32, i32), stronghold::Stronghold>,
    templates: templated::TemplateStarts,
}

impl Structures {
    /// Caches sized for `areas` areas generated at once (`OverworldGenerator::with_areas`).
    pub fn new(seed: i64, areas: usize) -> Self {
        Self::with_start_limit(seed, areas, areas * STARTS_LIMIT)
    }

    /// `starts` starts of each kind kept, whatever `areas` (eviction probes: examples/walk.rs).
    pub fn with_start_limit(seed: i64, areas: usize, starts: usize) -> Self {
        let level_seed = seed as u32;
        let mineshaft_seed = placement::MineshaftSeed::new(level_seed);
        Self {
            seed,
            level_seed,
            mineshaft_seed,
            mineshaft_chunks: StartCache::new(areas * CHECKS_LIMIT),
            grid_starts: StartCache::new(areas * CHECKS_LIMIT),
            strongholds: OnceLock::new(),
            mineshafts: StartCache::new(starts),
            monuments: StartCache::new(starts),
            stronghold_pieces: StartCache::new(starts),
            templates: templated::TemplateStarts::new(starts),
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
            StructureKind::Mineshaft => *self.mineshaft_chunks.get(chunk, || self.mineshaft_seed.is_start(chunk)),
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
            StructureKind::TrialChambers | StructureKind::TrailRuins | StructureKind::AbandonedCamp => self.v2_starts(density, kind, chunk),
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

    /// Entries each start map holds, for memory probes (examples/walk.rs).
    /// Entries each start map holds and has made, for memory and eviction probes (examples/walk.rs).
    pub fn cache_counts(&self) -> Vec<(&'static str, usize, usize)> {
        fn count<K: std::hash::Hash + Eq + Copy, V>(name: &'static str, cache: &StartCache<K, V>) -> (&'static str, usize, usize) {
            let (kept, made) = cache.counts();
            (name, kept, made)
        }
        let t = &self.templates;
        vec![
            count("mineshaft_chunks", &self.mineshaft_chunks),
            count("grid_starts", &self.grid_starts),
            count("mineshafts", &self.mineshafts),
            count("monuments", &self.monuments),
            count("strongholds", &self.stronghold_pieces),
            count("ocean_ruins", &t.ocean_ruins),
            count("outposts", &t.outposts),
            count("cities", &t.ancient_cities),
            count("villages", &t.villages),
            count("mansions", &t.mansions),
            count("v2", &t.v2),
        ]
    }

    fn strongholds(&self, density: &Density) -> &[(i32, i32)] {
        self.strongholds.get_or_init(|| stronghold::spiral(self.level_seed, &|chunk| self.is_start(density, StructureKind::Village, chunk)))
    }
}
