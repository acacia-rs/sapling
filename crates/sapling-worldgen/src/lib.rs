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
mod structured;
mod surface;
mod terrain;

use std::sync::Arc;

pub use biome::ChunkBiomes;
pub use block::Block;
#[doc(hidden)]
pub use feature::reach::{Cells as ReachCells, Tries as ReachTries};
#[doc(hidden)]
pub use structured::replay::{ReplayWrite, StructureStep};
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
    structures: structure::Structures,
    carved: SourceCache<(i32, i32), ChunkShape>,
    generated_tops: SourceCache<(i32, i32), [i16; 256]>,
    pieced: SourceCache<(i32, i32), structured::Pieced>,
    structure_replay: Option<structured::replay::Replay>,
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
            generated_tops: SourceCache::new(LOADED_LIMIT),
            pieced: SourceCache::new(LOADED_LIMIT),
            structure_replay: None,
            loaded: SourceCache::new(LOADED_LIMIT),
            biomes: SourceCache::new(BIOMES_LIMIT),
        }
    }

    /// Every ported stage: `loaded_blocks`, then `place_features` (lava lakes included).
    pub fn chunk_blocks(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        let mut shape = (*self.loaded(chunk_x, chunk_z)).clone();
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
            let (mut shape, tops) = self.surfaced(chunk_x, chunk_z);
            self.generated_tops.get((chunk_x, chunk_z), || tops);
            self.carve(&mut shape, chunk_x, chunk_z);
            shape
        })
    }

    /// The pre-worldgen heightmap (`LevelChunk` +0xf80, filled by `_prepareHeights`): `ChunkShape::tops` of the
    /// terrain before its surface and carvers.
    fn generated_tops(&self, chunk_x: i32, chunk_z: i32) -> Arc<[i16; 256]> {
        self.generated_tops.get((chunk_x, chunk_z), || self.surfaced(chunk_x, chunk_z).1)
    }

    /// The terrain stages in BDS's `loadChunk` order, up to (not including) carvers.
    pub fn surfaced_blocks(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        self.surfaced(chunk_x, chunk_z).0
    }

    fn surfaced(&self, chunk_x: i32, chunk_z: i32) -> (ChunkShape, [i16; 256]) {
        let adjustments = self.structures.adjustments(&self.density, (chunk_x, chunk_z));
        let mut shape = self.density.adjusted_chunk_shape(chunk_x, chunk_z, (!adjustments.is_empty()).then_some(&adjustments));
        let tops = shape.tops();
        let mut random = floor::chunk_random(chunk_x, chunk_z);
        floor::place_bedrock_floor(&mut shape, &mut random);
        let biomes = self.chunk_biomes(chunk_x, chunk_z);
        self.surface.build_chunk_surface(&mut shape, &biomes, &self.density, (chunk_x, chunk_z), &mut random);
        (shape, tops)
    }

    /// Density, aquifers, deepslate and ore veins only.
    pub fn chunk_shape(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        self.density.chunk_shape(chunk_x, chunk_z)
    }

    /// The legacy carvers (caves, canyons), applied to a chunk after its surface.
    pub fn carve(&self, shape: &mut ChunkShape, chunk_x: i32, chunk_z: i32) {
        self.carvers.carve(&self.density, shape, chunk_x, chunk_z);
    }

    /// The lava lakes alone, each over its chunks as loaded: a stage for the oracle's judge and probes.
    /// The generated chunk gets its lakes from `place_features` (feature/lake.rs).
    pub fn decorate(&self, shape: &mut ChunkShape, chunk_x: i32, chunk_z: i32) {
        self.decoration.decorate(shape, chunk_x, chunk_z, &|x, z| self.loaded(x, z));
    }

    /// The decoration of every chunk that reaches this one: its lava lake, monster rooms, feature rules and liquid.
    pub fn place_features(&self, shape: &mut ChunkShape, chunk_x: i32, chunk_z: i32) {
        self.with_inputs(|inputs| self.features.place(shape, (chunk_x, chunk_z), inputs));
    }

    /// One source chunk's feature writes, decorated alone: world (x, y, z), block placed, block replaced.
    #[doc(hidden)]
    pub fn source_feature_writes(&self, chunk_x: i32, chunk_z: i32) -> Vec<([i32; 3], Block, Block)> {
        let writes = self.with_inputs(|inputs| self.features.source_writes((chunk_x, chunk_z), inputs));
        writes.world_writes((chunk_x, chunk_z))
    }

    /// From now on sources follow what BDS's thread-local generator could reach (feature/reach.rs).
    #[doc(hidden)]
    pub fn track_reach(&mut self) {
        self.features.track_reach = true;
    }

    /// Decorates in the order one BDS run did (chunks as it started them) instead of by phase; chunks
    /// the order lacks decorate nothing. For checking a live run (scripts/dbg_order.py).
    #[doc(hidden)]
    /// `thread`: the run had one worker whose generator was seeded so (scripts/bds_seed0.py) and had
    /// drawn that often at each chunk's first `MultifaceFeature` draw (scripts/dbg_loads.py).
    pub fn set_decoration_order(&mut self, order: &[(i32, i32)], thread: Option<(u32, &[Option<u32>])>) {
        self.features.set_order(order, thread);
    }

    /// The chunk's cells a thread-local draw of BDS could have left different from ours, and the
    /// tries the chunk followed as a source; needs `track_reach` before anything is generated.
    #[doc(hidden)]
    pub fn thread_local_reach(&self, chunk_x: i32, chunk_z: i32) -> (Option<ReachCells>, ReachTries) {
        self.with_inputs(|inputs| self.features.reach((chunk_x, chunk_z), inputs))
    }

    /// Molang's `query.noise(x, z)` (examples/query_noise.rs).
    #[doc(hidden)]
    pub fn query_noise(x: f32, z: f32) -> f32 {
        feature::query_noise(x, z)
    }

    /// The feature trace of the replayed structure stage (pieces' features: `element` lines and their writes);
    /// empty once the stage has run.
    #[doc(hidden)]
    pub fn structure_replay_trace(&self) -> Vec<String> {
        feature::trace::capture(|| self.structure_replay_writes().len()).1
    }

    /// `source_feature_writes` with the decoration trace (feature/trace.rs) of every source it ran.
    #[doc(hidden)]
    pub fn source_feature_trace(&self, chunk_x: i32, chunk_z: i32) -> Vec<String> {
        feature::trace::capture(|| self.source_feature_writes(chunk_x, chunk_z)).1
    }

    fn with_inputs<T>(&self, run: impl FnOnce(feature::Inputs) -> T) -> T {
        let loaded = |x, z| self.loaded(x, z);
        let biomes = |x, z| self.chunk_biomes(x, z);
        let surface = |qx, qz| self.density.preliminary_surface(qx, qz);
        let in_structure = |[x, y, z]: [i32; 3]| self.structures.in_structure(&self.density, (x, y, z), true);
        let in_tunnels = |[x, y, z]: [i32; 3]| self.structures.in_structure(&self.density, (x, y, z), false);
        let carved = |x, z| self.carved(x, z);
        let generated_tops = |x, z| self.generated_tops(x, z);
        let inputs = feature::Inputs { loaded: &loaded, carved: &carved, generated_tops: &generated_tops, biomes: &biomes, surface: &surface,in_structure: &in_structure, in_tunnels: &in_tunnels };
        run(inputs)
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

    /// Whether structure terrain adjustments reach the chunk, for probes.
    #[doc(hidden)]
    pub fn has_terrain_adjustments(&self, chunk_x: i32, chunk_z: i32) -> bool {
        !self.structures.adjustments(&self.density, (chunk_x, chunk_z)).is_empty()
    }

    #[doc(hidden)]
    pub fn village_pieces(&self, chunk_x: i32, chunk_z: i32) -> Vec<String> {
        self.structures.village_pieces(&self.density, (chunk_x, chunk_z))
    }

    #[doc(hidden)]
    pub fn mineshaft_pieces(&self, chunk_x: i32, chunk_z: i32) -> Vec<(String, structure::BoundingBox)> {
        self.structures.mineshaft_pieces(&self.density, (chunk_x, chunk_z))
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

    #[doc(hidden)]
    pub fn v2_template_cells(&self, chunk_x: i32, chunk_z: i32) -> Vec<([i32; 3], String, Block, u8, Block)> {
        self.structures.v2_template_cells(&self.density, (chunk_x, chunk_z))
    }

    /// For probes, process-wide and once: the element feature `from` places `to` from here on.
    #[doc(hidden)]
    pub fn swap_feature(from: &str, to: &str) -> bool {
        feature::swap_feature(from, to)
    }

    #[doc(hidden)]
    pub fn v2_pieces(&self, chunk_x: i32, chunk_z: i32) -> Vec<String> {
        self.structures.v2_pieces(&self.density, (chunk_x, chunk_z))
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
