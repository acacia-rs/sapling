//! BDS's data-driven feature rules (`feature_rules`/`features` JSON, transcribed into data.rs):
//! each source chunk is decorated alone over its neighbours' terrain, and a column replays the
//! writes of every source chunk that reaches it (docs/research/worldgen-bds.md, Feature placement).

mod aquatic;
mod data;
mod decorate;
mod disk;
mod flowers;
mod land;
mod molang;
mod ore;
mod place;
mod plants;
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
/// Chunks as earlier phases left them, about 200 kB each.
const BEFORE_LIMIT: usize = 768;

type Load<'a> = &'a dyn Fn(i32, i32) -> Arc<ChunkShape>;
type Biomes<'a> = &'a dyn Fn(i32, i32) -> Arc<ChunkBiomes>;

/// When BDS decorates a chunk relative to its neighbours: x and z even first, then x odd, both odd,
/// z odd. Chunks of one phase are never neighbours (docs/research/worldgen-bds-features.md).
fn phase((x, z): (i32, i32)) -> u8 {
    [0, 1, 3, 2][((x & 1) | (z & 1) << 1) as usize]
}

/// The sources whose features reach `target`, in decoration order, those of `phase` and later left out.
fn sources_before(target: (i32, i32), phase_limit: u8) -> Vec<(i32, i32)> {
    let mut sources: Vec<(i32, i32)> = (target.0 - REACH..=target.0 + REACH)
        .flat_map(|x| (target.1 - REACH..=target.1 + REACH).map(move |z| (x, z)))
        .filter(|&source| phase(source) < phase_limit)
        .collect();
    sources.sort_by_key(|&source| (phase(source), source));
    sources
}

pub struct Features {
    seed: LargeFeatureSeed,
    writes: SourceCache<(i32, i32), ChunkWrites>,
    before: SourceCache<(i32, i32, u8), ChunkShape>,
}

impl Features {
    /// Seeded like the carvers: `(x*a + z*b) ^ seed` on the low 32 bits of the world seed.
    pub fn new(world_seed: i64) -> Self {
        Self { seed: LargeFeatureSeed::new(world_seed as u32), writes: SourceCache::new(CACHE_LIMIT), before: SourceCache::new(BEFORE_LIMIT) }
    }

    /// `loaded` gives a chunk's terrain before features, `biomes` its biomes. Sources apply in
    /// decoration order; a write lands when the block it replaced in its source's view is still there.
    pub fn place(&self, shape: &mut ChunkShape, chunk: (i32, i32), loaded: Load, biomes: Biomes) {
        self.replay(shape, chunk, u8::MAX, loaded, biomes);
    }

    fn replay(&self, shape: &mut ChunkShape, chunk: (i32, i32), phase_limit: u8, loaded: Load, biomes: Biomes) {
        for source in sources_before(chunk, phase_limit) {
            let writes = self.source_writes(source, loaded, biomes);
            for write in writes.towards(chunk.0 - source.0, chunk.1 - source.1) {
                let index = write.index as usize;
                if shape.block_at(index) == write.seen {
                    shape.set_at(index, write.block);
                }
            }
        }
    }

    /// A chunk with the features of every source decorated before `phase_limit`.
    fn before(&self, chunk: (i32, i32), phase_limit: u8, loaded: Load, biomes: Biomes) -> Arc<ChunkShape> {
        if phase_limit == 0 {
            return loaded(chunk.0, chunk.1);
        }
        self.before.get((chunk.0, chunk.1, phase_limit), || {
            let mut shape = (*loaded(chunk.0, chunk.1)).clone();
            self.replay(&mut shape, chunk, phase_limit, loaded, biomes);
            shape
        })
    }

    /// One source chunk's writes over its neighbours as the earlier phases left them.
    pub fn source_writes(&self, source: (i32, i32), loaded: Load, biomes: Biomes) -> Arc<ChunkWrites> {
        self.writes.get(source, || {
            let phase_limit = phase(source);
            let base = |x: i32, z: i32| self.before((x, z), phase_limit, loaded, biomes);
            let mut view = View::new(source, &base);
            decorate::decorate_chunk(&self.seed, source, biomes, &mut view);
            view.into_writes()
        })
    }
}
