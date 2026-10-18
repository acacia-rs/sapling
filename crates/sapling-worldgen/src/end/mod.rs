//! The End as BDS's `TheEndGenerator` (constructor 1482414a0, vtable 14a924380, `loadChunk` 148242a00) makes it:
//! Java 1.12's End terrain on MT19937, one biome (docs/research/worldgen-bds-end.md).

mod terrain;

use std::sync::Arc;

use crate::biome::ChunkBiomes;
use crate::cache::SourceCache;
use crate::chunk::ChunkShape;

pub use terrain::HEIGHT;

/// `minecraft:the_end`, every column at every y.
pub const THE_END: u8 = 9;
const BIOMES_LIMIT: usize = 64;

pub struct EndGenerator {
    noises: terrain::EndNoises,
    chunk_biomes: SourceCache<(i32, i32), ChunkBiomes>,
}

impl EndGenerator {
    /// The terrain's MT takes the seed's low 32 bits.
    pub fn new(seed: i64) -> Self {
        Self { noises: terrain::EndNoises::new(seed as u32), chunk_biomes: SourceCache::new(BIOMES_LIMIT) }
    }

    /// End stone and air from the density alone.
    pub fn chunk_shape(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        self.noises.chunk_shape(chunk_x, chunk_z)
    }

    /// The buffer `loadChunk` commits (1482431de): the density's end stone. The surface builder `minecraft:the_end`
    /// (1487e4fd0) turns stone into end stone and finds none.
    pub fn terrain_blocks(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        self.chunk_shape(chunk_x, chunk_z)
    }

    /// Every ported stage.
    pub fn chunk_blocks(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        self.terrain_blocks(chunk_x, chunk_z)
    }

    pub fn chunk_biomes(&self, chunk_x: i32, chunk_z: i32) -> Arc<ChunkBiomes> {
        self.chunk_biomes.get((chunk_x, chunk_z), || ChunkBiomes::from_columns((chunk_x, chunk_z), &[THE_END; 256]))
    }

    /// The island height Java's `getIslandHeightValue` gives a chunk's cell column (0..3 each way).
    pub fn island_height(&self, chunk_x: i32, chunk_z: i32, cell_x: i32, cell_z: i32) -> f32 {
        self.noises.island_height(chunk_x, chunk_z, cell_x, cell_z)
    }
}
