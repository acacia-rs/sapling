//! Bedrock overworld terrain generation, ported from BDS 1.26.5x: the blocks of a chunk through
//! its surface, BDS's legacy carvers and lava lakes, its data-driven feature rules, and the biome
//! of every block. BDS's terrain is a C++ port of Java 1.18.1's `NoiseSampler` done in f32; this
//! crate follows BDS's arithmetic, not Java's. Everything after the terrain shape uses Bedrock's
//! MT19937 (docs/research/worldgen-bds.md).

pub mod biome;
mod block;
mod cache;
mod carver;
pub mod climate;
mod decoration;
pub mod density;
mod feature;
mod floor;
mod fxhash;
mod mth;
pub mod noise;
pub mod random;
pub mod shaper;
mod surface;
mod terrain;

use std::sync::Arc;

pub use biome::ChunkBiomes;
pub use block::Block;
pub use terrain::{CellKind, ChunkShape, HEIGHT, MIN_Y};

use cache::SourceCache;
use carver::Carvers;
use decoration::Decoration;
use density::Density;
use feature::Features;
use random::Xoroshiro;

/// Loaded columns (about 100 kB each) kept for neighbours' lakes and features.
const LOADED_LIMIT: usize = 256;
/// Chunk biomes (about 200 kB each), shared by the surface, decoration and the caller.
const BIOMES_LIMIT: usize = 256;

/// The overworld generator for one world seed.
pub struct OverworldGenerator {
    seed: i64,
    density: Density,
    surface: surface::SurfaceNoises,
    carvers: Carvers,
    decoration: Decoration,
    features: Features,
    loaded: SourceCache<(i32, i32), ChunkShape>,
    biomes: SourceCache<(i32, i32), ChunkBiomes>,
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
            features: Features::new(seed),
            loaded: SourceCache::new(LOADED_LIMIT),
            biomes: SourceCache::new(BIOMES_LIMIT),
        }
    }

    /// Every ported stage: `loaded_blocks`, `decorate`, then `place_features`.
    pub fn chunk_blocks(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        let mut shape = (*self.loaded(chunk_x, chunk_z)).clone();
        self.decorate(&mut shape, chunk_x, chunk_z);
        self.place_features(&mut shape, chunk_x, chunk_z);
        shape
    }

    /// `loaded_blocks`, kept for the neighbours that read it.
    fn loaded(&self, chunk_x: i32, chunk_z: i32) -> Arc<ChunkShape> {
        self.loaded.get((chunk_x, chunk_z), || self.loaded_blocks(chunk_x, chunk_z))
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
        self.decoration.decorate(shape, chunk_x, chunk_z, &|x, z| self.loaded(x, z));
    }

    /// The feature rules of every chunk whose features reach this one, applied after `decorate`.
    pub fn place_features(&self, shape: &mut ChunkShape, chunk_x: i32, chunk_z: i32) {
        self.with_inputs(|inputs| self.features.place(shape, (chunk_x, chunk_z), inputs));
    }

    /// One source chunk's feature writes, decorated alone: world (x, y, z), block placed, block replaced.
    #[doc(hidden)]
    pub fn source_feature_writes(&self, chunk_x: i32, chunk_z: i32) -> Vec<([i32; 3], Block, Block)> {
        let writes = self.with_inputs(|inputs| self.features.source_writes((chunk_x, chunk_z), inputs));
        writes.world_writes((chunk_x, chunk_z))
    }

    fn with_inputs<T>(&self, run: impl FnOnce(feature::Inputs) -> T) -> T {
        let loaded = |x, z| self.loaded(x, z);
        let biomes = |x, z| self.chunk_biomes(x, z);
        let surface = |qx, qz| self.density.preliminary_surface(qx, qz);
        let after_lake = |x, z| self.decoration.after_lake(x, z, &|cx, cz| self.loaded(cx, cz));
        run(feature::Inputs { loaded: &loaded, biomes: &biomes, surface: &surface, after_lake: &after_lake })
    }

    /// Cave segments near a column, for probes (`Carvers::segments_near`).
    #[doc(hidden)]
    pub fn cave_segments_near(&self, x: i32, z: i32, reach: f32, skip_gate: bool) -> Vec<(usize, [f32; 3], f32, f32)> {
        self.carvers.segments_near(x, z, reach, skip_gate)
    }

    /// Bedrock biome ids per block, cached for the stages and neighbours that read them again.
    pub fn chunk_biomes(&self, chunk_x: i32, chunk_z: i32) -> Arc<ChunkBiomes> {
        self.biomes.get((chunk_x, chunk_z), || self.density.climate.chunk_biomes(self.seed, chunk_x, chunk_z))
    }

    pub fn density(&self) -> &Density {
        &self.density
    }
}
