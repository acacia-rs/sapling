//! The Nether as BDS's `NetherGenerator` (constructor 1481e6d60, vtable 14a923740, `loadChunk` 1481ec000)
//! makes it: Java 1.12's hell terrain and a legacy layer biome stack, both on MT19937, then the bedrock
//! and the biomes' surface builders (docs/research/worldgen-bds-nether.md).

mod biome;
mod floor;
mod terrain;

use std::sync::Arc;

use crate::biome::ChunkBiomes;
use crate::cache::SourceCache;
use crate::chunk::ChunkShape;
use crate::random::Mt19937;

pub use biome::{BASALT_DELTAS, CRIMSON_FOREST, HELL, SOULSAND_VALLEY, WARPED_FOREST};
pub use terrain::{HEIGHT, LAVA_LEVEL};

const BIOMES_LIMIT: usize = 256;

pub struct NetherGenerator {
    noises: terrain::NetherNoises,
    biomes: biome::NetherBiomes,
    chunk_biomes: SourceCache<(i32, i32), ChunkBiomes>,
}

impl NetherGenerator {
    /// The terrain's MT takes the seed's low 32 bits.
    pub fn new(seed: i64) -> Self {
        Self { noises: terrain::NetherNoises::new(seed as u32), biomes: biome::NetherBiomes::new(seed), chunk_biomes: SourceCache::new(BIOMES_LIMIT) }
    }

    /// Netherrack, lava and air from the density alone.
    pub fn chunk_shape(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        self.noises.chunk_shape(chunk_x, chunk_z)
    }

    /// The density, then the bedrock.
    pub fn bedrock_blocks(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        self.bedrocked(chunk_x, chunk_z).0
    }

    /// The bedrock on the density, and the chunk MT where the bedrock left it.
    fn bedrocked(&self, chunk_x: i32, chunk_z: i32) -> (ChunkShape, Mt19937) {
        let mut shape = self.chunk_shape(chunk_x, chunk_z);
        let mut random = floor::chunk_random(chunk_x, chunk_z);
        floor::place_bedrock(&mut shape, &mut random);
        (shape, random)
    }

    /// `bedrock_blocks`, then the biomes' surface builders.
    pub fn surfaced_blocks(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        self.bedrocked(chunk_x, chunk_z).0
    }

    /// `surfaced_blocks`, then the pregeneration pass's carvers: the buffer `loadChunk` commits (1481eca38).
    pub fn carved_blocks(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        self.surfaced_blocks(chunk_x, chunk_z)
    }

    /// Bedrock biome ids, one per column at every y.
    pub fn chunk_biomes(&self, chunk_x: i32, chunk_z: i32) -> Arc<ChunkBiomes> {
        self.chunk_biomes.get((chunk_x, chunk_z), || {
            let columns = std::array::from_fn(|i| self.biomes.biome(chunk_x * 16 + (i % 16) as i32, chunk_z * 16 + (i / 16) as i32));
            ChunkBiomes::from_columns((chunk_x, chunk_z), &columns)
        })
    }

    /// The multi-noise layer's biome at a quart column, before the voronoi zoom.
    #[doc(hidden)]
    pub fn quart_biome(&self, qx: i32, qz: i32) -> u8 {
        self.biomes.quart_biome(qx, qz)
    }
}
