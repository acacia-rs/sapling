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
pub mod structure;
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

/// A chunk after its structure pieces, and their features' writes into its neighbours.
type Pieced = (ChunkShape, Vec<([i32; 3], Block, Block)>);

/// The overworld generator for one world seed.
pub struct OverworldGenerator {
    seed: i64,
    density: Density,
    surface: surface::SurfaceNoises,
    carvers: Carvers,
    decoration: Decoration,
    features: Features,
    structures: structure::Structures,
    carved: SourceCache<(i32, i32), ChunkShape>,
    pieced: SourceCache<(i32, i32), Pieced>,
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
            structures: structure::Structures::new(seed),
            carved: SourceCache::new(LOADED_LIMIT),
            pieced: SourceCache::new(LOADED_LIMIT),
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

    /// BDS's `loadChunk` (`surfaced_blocks`, then `carve`) and its structure post-process
    /// (`place_structures`): what decoration starts from, here and in the neighbours.
    pub fn loaded_blocks(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        let mut shape = (*self.carved(chunk_x, chunk_z)).clone();
        self.place_structures(&mut shape, chunk_x, chunk_z);
        shape
    }

    /// `surfaced_blocks` then `carve`, kept for the structure stage's reads past its chunk.
    fn carved(&self, chunk_x: i32, chunk_z: i32) -> Arc<ChunkShape> {
        self.carved.get((chunk_x, chunk_z), || {
            let mut shape = self.surfaced_blocks(chunk_x, chunk_z);
            self.carve(&mut shape, chunk_x, chunk_z);
            shape
        })
    }

    /// The terrain stages in BDS's `loadChunk` order, up to (not including) carvers.
    pub fn surfaced_blocks(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        let adjustments = self.structures.adjustments(&self.density, (chunk_x, chunk_z));
        let mut shape = self.density.adjusted_chunk_shape(chunk_x, chunk_z, (!adjustments.is_empty()).then_some(&adjustments));
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

    /// The structure stage (`structurePostProcessChunk`, after carvers and before decoration):
    /// the pieces of every structure reaching the chunk, written inside it only.
    pub fn place_structures(&self, shape: &mut ChunkShape, chunk_x: i32, chunk_z: i32) {
        let mut heights = self.place_pieces(shape, chunk_x, chunk_z).map(|placed| placed.touched);
        for (sx, sz) in self.structures.feature_sources(&self.density, (chunk_x, chunk_z)) {
            let source = self.pieced(sx, sz);
            for &([x, y, z], block, seen) in source.1.iter().filter(|(p, ..)| (p[0] >> 4, p[2] >> 4) == (chunk_x, chunk_z)) {
                let (lx, lz) = ((x & 15) as usize, (z & 15) as usize);
                if shape.block(lx, y, lz) == seen {
                    shape.set_block(lx, y, lz, block);
                    heights = Some(heights.map_or((y, y), |(lo, hi)| (lo.min(y), hi.max(y))));
                }
            }
        }
        if let Some(heights) = heights {
            let neighbour = |x: i32, y: i32, z: i32| self.pieced(x >> 4, z >> 4).0.block((x & 15) as usize, y, (z & 15) as usize);
            structure::rejoin_connections(shape, (chunk_x, chunk_z), heights, &neighbour);
        }
    }

    /// The structure pieces alone, reading past the chunk through the neighbours' carved terrain.
    fn place_pieces(&self, shape: &mut ChunkShape, chunk_x: i32, chunk_z: i32) -> Option<structure::Placed> {
        let neighbour = |x: i32, y: i32, z: i32| self.carved(x >> 4, z >> 4).block((x & 15) as usize, y, (z & 15) as usize);
        let biomes = self.chunk_biomes(chunk_x, chunk_z);
        let load = |x: i32, z: i32| self.carved(x, z);
        let biome = |[x, y, z]: [i32; 3]| Some(self.chunk_biomes(x >> 4, z >> 4).biome((x & 15) as usize, y, (z & 15) as usize));
        let surface = |qx: i32, qz: i32| self.density.preliminary_surface(qx, qz);
        let features = feature::ElementWorld { load: &load, biome: &biome, surface: &surface };
        self.structures.place(&self.density, shape, (chunk_x, chunk_z), &neighbour, &biomes, &features)
    }

    /// `carved` plus the structure pieces, for the neighbours' fence connections, with the writes
    /// its pieces' features made past it.
    fn pieced(&self, chunk_x: i32, chunk_z: i32) -> Arc<Pieced> {
        self.pieced.get((chunk_x, chunk_z), || {
            let mut shape = (*self.carved(chunk_x, chunk_z)).clone();
            let spilled = self.place_pieces(&mut shape, chunk_x, chunk_z).map_or_else(Vec::new, |placed| placed.spilled);
            (shape, spilled)
        })
    }

    /// Structures that may place blocks in the chunk: kind, start chunk, and a box around what
    /// the structure covers (its pieces where ported, a rough area otherwise).
    pub fn structure_areas(&self, chunk_x: i32, chunk_z: i32) -> Vec<(structure::StructureKind, (i32, i32), structure::BoundingBox)> {
        self.structures.areas_touching(&self.density, (chunk_x, chunk_z))
    }

    /// Structure pieces' features in the 3x3 chunks around one, for probes: feature, origin.
    #[doc(hidden)]
    pub fn structure_features_near(&self, chunk_x: i32, chunk_z: i32) -> Vec<(String, (i32, i32, i32))> {
        self.structures.feature_pieces(&self.density, (chunk_x, chunk_z))
    }

    #[doc(hidden)]
    pub fn village_pieces(&self, chunk_x: i32, chunk_z: i32) -> Vec<String> {
        self.structures.village_pieces(&self.density, (chunk_x, chunk_z))
    }

    /// A structure feature placed alone from a chunk over its carved terrain, its structure
    /// generator `skip` draws in, for probes: (position, block, block replaced).
    #[doc(hidden)]
    pub fn structure_feature_writes(&self, name: &str, chunk: (i32, i32), at: [i32; 3], skip: usize) -> Vec<([i32; 3], Block, Block)> {
        let mut random = random::Mt19937::new(random::LargeFeatureSeed::new(self.seed as u32).chunk(chunk.0, chunk.1));
        (0..skip).for_each(|_| _ = random.next_u32());
        let load = |x: i32, z: i32| self.carved(x, z);
        let biome = |[x, y, z]: [i32; 3]| Some(self.chunk_biomes(x >> 4, z >> 4).biome((x & 15) as usize, y, (z & 15) as usize));
        let surface = |qx: i32, qz: i32| self.density.preliminary_surface(qx, qz);
        let world = feature::ElementWorld { load: &load, biome: &biome, surface: &surface };
        feature::place_element(name, chunk, self.carved(chunk.0, chunk.1), &world, &mut random, at).unwrap_or_default()
    }

    /// Whether a structure of `kind` starts in the chunk.
    pub fn is_structure_start(&self, kind: structure::StructureKind, chunk_x: i32, chunk_z: i32) -> bool {
        self.structures.is_start(&self.density, kind, (chunk_x, chunk_z))
    }

    /// The start chunk `/locate structure` answers from a block position (grid-placed kinds).
    pub fn locate_structure(&self, kind: structure::StructureKind, x: i32, z: i32) -> Option<(i32, i32)> {
        self.structures.locate(&self.density, kind, (x, z))
    }

    pub fn density(&self) -> &Density {
        &self.density
    }
}
