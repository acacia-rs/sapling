//! BDS's data-driven feature rules (`feature_rules`/`features` JSON, transcribed into data.rs):
//! each source chunk is decorated alone over its neighbours' terrain, and a column replays the
//! writes of every source chunk that reaches it (docs/research/worldgen-bds.md, Feature placement).

mod data;
mod decorate;
mod molang;
mod ore;
mod scatter;
mod spec;
mod view;

use std::sync::Arc;

use crate::biome::ChunkBiomes;
use crate::cache::SourceCache;
use crate::random::LargeFeatureSeed;
use crate::terrain::ChunkShape;

use view::{ChunkWrites, REACH, View};

/// Source chunks' writes are a few hundred kB each.
const CACHE_LIMIT: usize = 1024;

pub struct Features {
    seed: LargeFeatureSeed,
    writes: SourceCache<(i32, i32), ChunkWrites>,
}

impl Features {
    /// Seeded like the carvers: `(x*a + z*b) ^ seed` on the low 32 bits of the world seed.
    pub fn new(world_seed: i64) -> Self {
        Self { seed: LargeFeatureSeed::new(world_seed as u32), writes: SourceCache::new(CACHE_LIMIT) }
    }

    /// `loaded` gives a chunk's terrain before features, `biomes` its biomes. A write lands when the
    /// block it replaced in its source's view is still there.
    pub fn place(
        &self,
        shape: &mut ChunkShape,
        (chunk_x, chunk_z): (i32, i32),
        loaded: &dyn Fn(i32, i32) -> Arc<ChunkShape>,
        biomes: &dyn Fn(i32, i32) -> ChunkBiomes,
    ) {
        for source_x in chunk_x - REACH..=chunk_x + REACH {
            for source_z in chunk_z - REACH..=chunk_z + REACH {
                let writes = self.writes.get((source_x, source_z), || {
                    let mut view = View::new((source_x, source_z), loaded);
                    decorate::decorate_chunk(&self.seed, (source_x, source_z), &biomes(source_x, source_z), &mut view);
                    view.into_writes()
                });
                for write in writes.towards(chunk_x - source_x, chunk_z - source_z) {
                    let index = write.index as usize;
                    if shape.block_at(index) == write.seen {
                        shape.set_at(index, write.block);
                    }
                }
            }
        }
    }
}
