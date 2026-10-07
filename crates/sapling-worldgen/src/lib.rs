//! Bedrock overworld terrain generation, ported from BDS 1.26.5x: the blocks of a chunk up to its
//! surface, and the biome of every block. BDS's terrain is a C++ port of Java 1.18.1's
//! `NoiseSampler` done in f32; this crate follows BDS's arithmetic, not Java's
//! (docs/research/worldgen-bds.md).

pub mod biome;
mod block;
pub mod climate;
pub mod density;
mod floor;
pub mod noise;
pub mod random;
pub mod shaper;
mod surface;
mod terrain;

pub use biome::ChunkBiomes;
pub use block::Block;
pub use terrain::{CellKind, ChunkShape, HEIGHT, MIN_Y};

use density::Density;
use random::Xoroshiro;

/// The overworld generator for one world seed.
pub struct OverworldGenerator {
    seed: i64,
    density: Density,
    surface: surface::SurfaceNoises,
}

impl OverworldGenerator {
    pub fn new(seed: i64) -> Self {
        let factory = Xoroshiro::from_seed(seed).fork_positional();
        Self { seed, density: Density::new(&factory), surface: surface::SurfaceNoises::new(&factory, seed) }
    }

    /// The terrain stages in BDS's `loadChunk` order, up to (not including) carvers and features.
    pub fn chunk_blocks(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        let mut shape = self.density.chunk_shape(chunk_x, chunk_z);
        let mut random = floor::chunk_random(chunk_x, chunk_z);
        floor::place_bedrock_floor(&mut shape, &mut random);
        let biomes = self.chunk_biomes(chunk_x, chunk_z);
        self.surface.build_chunk_surface(&mut shape, &biomes, &self.density, (chunk_x, chunk_z), &mut random);
        shape
    }

    /// Density, aquifers, deepslate and ore veins only.
    pub fn chunk_shape(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        self.density.chunk_shape(chunk_x, chunk_z)
    }

    /// Bedrock biome ids per block.
    pub fn chunk_biomes(&self, chunk_x: i32, chunk_z: i32) -> ChunkBiomes {
        self.density.climate.chunk_biomes(self.seed, chunk_x, chunk_z)
    }

    /// The surface material-adjustment noise at a block position and frequency scale.
    pub fn adjustment_noise(&self, position: [i32; 3], scale: f32) -> f32 {
        self.surface.adjustment(position, scale)
    }

    pub fn density(&self) -> &Density {
        &self.density
    }
}
