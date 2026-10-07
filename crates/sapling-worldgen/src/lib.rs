//! Bedrock overworld terrain generation, ported from BDS 1.26.5x: the blocks of a chunk through
//! its surface, BDS's legacy carvers and lava lakes, and the biome of every block. BDS's terrain
//! is a C++ port of Java 1.18.1's `NoiseSampler` done in f32; this crate follows BDS's
//! arithmetic, not Java's. Everything after the terrain shape uses Bedrock's MT19937
//! (docs/research/worldgen-bds.md).

pub mod biome;
mod block;
mod carver;
pub mod climate;
mod decoration;
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

use carver::Carvers;
use decoration::Decoration;
use density::Density;
use random::Xoroshiro;

/// The overworld generator for one world seed.
pub struct OverworldGenerator {
    seed: i64,
    density: Density,
    surface: surface::SurfaceNoises,
    carvers: Carvers,
    decoration: Decoration,
}

impl OverworldGenerator {
    pub fn new(seed: i64) -> Self {
        let factory = Xoroshiro::from_seed(seed).fork_positional();
        Self {
            seed,
            density: Density::new(&factory),
            surface: surface::SurfaceNoises::new(&factory, seed),
            carvers: Carvers::new(seed),
            decoration: Decoration::new(seed),
        }
    }

    /// Every ported stage: `loaded_blocks`, then `decorate`.
    pub fn chunk_blocks(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        let mut shape = self.loaded_blocks(chunk_x, chunk_z);
        self.decorate(&mut shape, chunk_x, chunk_z);
        shape
    }

    /// BDS's `loadChunk`: `surfaced_blocks`, then `carve`.
    pub fn loaded_blocks(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        let mut shape = self.surfaced_blocks(chunk_x, chunk_z);
        self.carve(&mut shape, chunk_x, chunk_z);
        shape
    }

    /// The terrain stages in BDS's `loadChunk` order, up to (not including) carvers.
    pub fn surfaced_blocks(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
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

    /// The legacy carvers (caves, canyons), applied to a chunk after its surface.
    pub fn carve(&self, shape: &mut ChunkShape, chunk_x: i32, chunk_z: i32) {
        self.carvers.carve(&self.density, shape, chunk_x, chunk_z);
    }

    /// The hardcoded post-process decoration (lava lakes), applied after `carve`.
    pub fn decorate(&self, shape: &mut ChunkShape, chunk_x: i32, chunk_z: i32) {
        self.decoration.decorate(shape, chunk_x, chunk_z, &|x, z| self.loaded_blocks(x, z));
    }

    /// Bedrock biome ids per block.
    pub fn chunk_biomes(&self, chunk_x: i32, chunk_z: i32) -> ChunkBiomes {
        self.density.climate.chunk_biomes(self.seed, chunk_x, chunk_z)
    }

    pub fn density(&self) -> &Density {
        &self.density
    }
}
