//! Bedrock overworld terrain generation, ported from BDS 1.26.5x: which cells are solid, air,
//! water or lava, and the biome of every 4x4x4 cell. BDS's terrain is a C++ port of Java
//! 1.18.1's `NoiseSampler` done in f32; this crate follows BDS's arithmetic, not Java's
//! (docs/research/worldgen-bds.md).

pub mod biome;
mod carver;
pub mod climate;
pub mod density;
pub mod noise;
pub mod random;
pub mod shaper;
mod terrain;

pub use biome::ChunkBiomes;
pub use terrain::{CellKind, ChunkShape, HEIGHT, MIN_Y};

use carver::Carvers;
use density::Density;
use random::Xoroshiro;

/// The overworld generator for one world seed.
pub struct OverworldGenerator {
    seed: i64,
    density: Density,
    carvers: Carvers,
}

impl OverworldGenerator {
    pub fn new(seed: i64) -> Self {
        let factory = Xoroshiro::from_seed(seed).fork_positional();
        Self { seed, density: Density::new(&factory), carvers: Carvers::new(seed) }
    }

    pub fn chunk_shape(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        self.density.chunk_shape(chunk_x, chunk_z)
    }

    /// The legacy carvers (caves, canyons), applied to a chunk's shape after its surface.
    pub fn carve(&self, shape: &mut ChunkShape, chunk_x: i32, chunk_z: i32) {
        self.carvers.carve(&self.density, shape, chunk_x, chunk_z);
    }

    /// Bedrock biome ids per block.
    pub fn chunk_biomes(&self, chunk_x: i32, chunk_z: i32) -> ChunkBiomes {
        self.density.climate.chunk_biomes(self.seed, chunk_x, chunk_z)
    }

    pub fn density(&self) -> &Density {
        &self.density
    }
}
