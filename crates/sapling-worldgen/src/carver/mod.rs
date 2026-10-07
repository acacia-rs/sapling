//! Legacy large-feature carvers, run on a chunk after its surface: the pregeneration-pass cave
//! carvers, then the hardcoded canyon. Each looks at the 17x17 source chunks around the target,
//! seeded by `LargeFeatureSeed`, and carves the parts of their caves that reach the target.

mod canyon;
mod cave;
mod carve;
mod replace;

use crate::cache::SourceCache;
use crate::density::{AquiferCache, Density};
use crate::random::{LargeFeatureSeed, Mt19937};
use crate::terrain::ChunkShape;

use canyon::{CANYON_1_18, Canyon, CanyonSettings};
use cave::{CaveSettings, EXTRA_CAVE_CARVER, Segment, UNDERGROUND_CAVE_CARVER};

const RANGE: i32 = 8;
/// Every feature rule reseeds with the level seed, so both carvers see the same chunk seeds.
const CAVE_CARVERS: [&CaveSettings; 2] = [&UNDERGROUND_CAVE_CARVER, &EXTRA_CAVE_CARVER];
const CANYON: &CanyonSettings = &CANYON_1_18;
/// BDS drops its cave cache every 1200 placements (148281d10).
const CACHE_LIMIT: usize = 16384;

pub struct Carvers {
    seed: LargeFeatureSeed,
    caves: SourceCache<(usize, i32, i32), Vec<Segment>>,
    canyons: SourceCache<(i32, i32), Option<Canyon>>,
}

impl Carvers {
    /// BDS seeds the decoration generator with the low 32 bits of the world seed.
    pub fn new(world_seed: i64) -> Self {
        let seed = LargeFeatureSeed::new(world_seed as u32);
        Self { seed, caves: SourceCache::new(CACHE_LIMIT), canyons: SourceCache::new(CACHE_LIMIT) }
    }

    pub fn carve(&self, density: &Density, shape: &mut ChunkShape, chunk_x: i32, chunk_z: i32) {
        let mut aquifer = AquiferCache::default();
        let chunk = (chunk_x, chunk_z);
        let sources = || (chunk_x - RANGE..=chunk_x + RANGE).flat_map(move |x| (chunk_z - RANGE..=chunk_z + RANGE).map(move |z| (x, z)));
        for (carver, settings) in CAVE_CARVERS.iter().enumerate() {
            for (x, z) in sources() {
                let segments = self.caves.get((carver, x, z), || settings.segments(&mut self.random(x, z), x, z));
                for segment in segments.iter().filter(|segment| carve::reaches(segment, chunk_x, chunk_z)) {
                    carve::carve(shape, density, &mut aquifer, segment, chunk, settings.height_limit);
                }
            }
        }
        for (x, z) in sources() {
            if let Some(canyon) = self.canyons.get((x, z), || CANYON.canyon(&mut self.random(x, z), x, z)).as_ref() {
                canyon.carve(shape, density, &mut aquifer, chunk, CANYON.max_y());
            }
        }
    }

    fn random(&self, chunk_x: i32, chunk_z: i32) -> Mt19937 {
        Mt19937::new(self.seed.chunk(chunk_x, chunk_z))
    }
}
