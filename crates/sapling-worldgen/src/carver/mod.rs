//! Legacy large-feature carvers, run on a chunk after its surface: the pregeneration-pass cave
//! carvers and the hardcoded canyon. Each looks at the 17x17 source chunks around the target,
//! seeded by `LargeFeatureSeed`, and carves the parts of their caves that reach the target.

mod cave;
mod carve;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::density::{AquiferCache, Density};
use crate::random::{LargeFeatureSeed, Mt19937};
use crate::terrain::ChunkShape;

use cave::{CaveSettings, EXTRA_CAVE_CARVER, Segment, UNDERGROUND_CAVE_CARVER};

const RANGE: i32 = 8;
/// BDS drops its segment cache every 1200 placements (148281d10); any bound works here.
const CACHE_LIMIT: usize = 16384;
/// Every feature rule reseeds with the level seed, so both carvers see the same chunk seeds.
const CAVE_CARVERS: [&CaveSettings; 2] = [&UNDERGROUND_CAVE_CARVER, &EXTRA_CAVE_CARVER];

pub struct Carvers {
    seed: LargeFeatureSeed,
    cave_segments: Mutex<HashMap<(usize, i32, i32), Arc<Vec<Segment>>>>,
}

impl Carvers {
    /// BDS seeds the decoration generator with the low 32 bits of the world seed.
    pub fn new(world_seed: i64) -> Self {
        Self { seed: LargeFeatureSeed::new(world_seed as u32), cave_segments: Mutex::default() }
    }

    pub fn carve(&self, density: &Density, shape: &mut ChunkShape, chunk_x: i32, chunk_z: i32) {
        let mut aquifer = AquiferCache::default();
        for (carver, settings) in CAVE_CARVERS.iter().enumerate() {
            for source_x in chunk_x - RANGE..=chunk_x + RANGE {
                for source_z in chunk_z - RANGE..=chunk_z + RANGE {
                    for segment in self.cave_segments(carver, source_x, source_z).iter() {
                        if carve::reaches(segment, chunk_x, chunk_z) {
                            carve::carve(shape, density, &mut aquifer, segment, (chunk_x, chunk_z), settings.height_limit);
                        }
                    }
                }
            }
        }
    }

    fn cave_segments(&self, carver: usize, chunk_x: i32, chunk_z: i32) -> Arc<Vec<Segment>> {
        let mut cache = self.cave_segments.lock().unwrap();
        if let Some(segments) = cache.get(&(carver, chunk_x, chunk_z)) {
            return segments.clone();
        }
        if cache.len() >= CACHE_LIMIT {
            cache.clear();
        }
        let mut random = Mt19937::new(self.seed.chunk(chunk_x, chunk_z));
        let segments = Arc::new(CAVE_CARVERS[carver].segments(&mut random, chunk_x, chunk_z));
        cache.insert((carver, chunk_x, chunk_z), segments.clone());
        segments
    }
}
