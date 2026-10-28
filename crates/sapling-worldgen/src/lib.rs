//! Bedrock overworld terrain generation, ported from BDS 1.26.5x: the blocks of a chunk through
//! its surface, BDS's legacy carvers and lava lakes, its data-driven feature rules, and the biome
//! of every block. BDS's terrain is a C++ port of Java 1.18.1's `NoiseSampler` done in f32; this
//! crate follows BDS's arithmetic, not Java's. Everything after the terrain shape uses Bedrock's
//! MT19937 (docs/research/worldgen-bds.md). The Nether's generator is `nether::NetherGenerator`
//! (docs/research/worldgen-bds-nether.md), the End's `end::EndGenerator` (docs/research/worldgen-bds-end.md).

pub mod biome;
mod block;
mod block_entity;
mod cache;
mod carver;
mod chunk;
pub mod climate;
mod decoration;
pub mod density;
pub mod end;
mod feature;
mod floor;
mod fxhash;
pub mod mth;
pub mod nether;
pub mod noise;
pub mod random;
pub mod shaper;
pub mod structure;
mod structure_queries;
mod structured;
mod surface;
mod terrain;

use std::sync::Arc;

pub use biome::ChunkBiomes;
pub use block::Block;
pub use block_entity::BlockEntity;
pub use end::EndGenerator;
pub use nether::NetherGenerator;
#[doc(hidden)]
pub use feature::grow::{is_feature, place_feature};
pub use feature::reach::{Cells as ReachCells, Tries as ReachTries};

/// BDS's `SculkSpreader` for a live world: a sculk catalyst's cursors (docs/research/sculk.md, "Catalyst").
pub mod sculk {
    pub use crate::feature::sculk::{Cursor, SculkWorld, Sound, Spreader, add_cursors, update_cursors};
    pub use crate::feature::spreader::Cells;
}
#[doc(hidden)]
pub use structured::StructureWrites;
pub use structured::replay::{ReplayWrite, StructureStep};
pub use structure::{PlaceFailure, StructureKind};
pub use surface::temperature;
pub use terrain::{CellKind, ChunkShape, HEIGHT, MIN_Y};

use cache::SourceCache;
use carver::Carvers;
use decoration::Decoration;
use density::Density;
use feature::Features;
use random::Xoroshiro;

/// Which of BDS's generators a stage follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dimension {
    Overworld,
    Nether,
    End,
}

/// Loaded columns (about 100 kB each) kept for neighbours' lakes and features.
const LOADED_LIMIT: usize = 256;
/// Post-processes kept (most are None: no start reaches the chunk), and ten times as many chunk states,
/// which share their blocks until a post-process writes them.
const PIECED_LIMIT: usize = 1024;
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
    pieced: SourceCache<(i32, i32), Option<structured::Source>>,
    staged: SourceCache<(i32, i32, u8), structured::Pieced>,
    structure_order: structured::order::Order,
    loaded: SourceCache<(i32, i32), ChunkShape>,
    biomes: SourceCache<(i32, i32), ChunkBiomes>,
}

impl OverworldGenerator {
    pub fn new(seed: i64) -> Self {
        Self::with_areas(seed, 1)
    }

    /// A generator whose caches hold `areas` separate areas generated at once (a server's workers
    /// each sweeping their own): sized for one, areas apart evict each other's terrain.
    pub fn with_areas(seed: i64, areas: usize) -> Self {
        let factory = Xoroshiro::from_seed(seed).fork_positional();
        let areas = areas.max(1);
        Self {
            seed,
            density: Density::new(&factory),
            surface: surface::SurfaceNoises::new(&factory, seed),
            carvers: Carvers::new(seed),
            decoration: Decoration::new(seed),
            features: Features::new(seed, areas),
            structures: structure::Structures::new(seed, areas),
            carved: SourceCache::new(areas * LOADED_LIMIT),
            generated_tops: SourceCache::new(areas * LOADED_LIMIT),
            pieced: SourceCache::new(areas * PIECED_LIMIT),
            staged: SourceCache::new(areas * 10 * PIECED_LIMIT),
            structure_order: Default::default(),
            loaded: SourceCache::new(areas * LOADED_LIMIT),
            biomes: SourceCache::new(areas * BIOMES_LIMIT),
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
        shape.share();
        shape
    }

    /// `surfaced_blocks` then `carve`, kept for the structure stage's reads past its chunk.
    fn carved(&self, chunk_x: i32, chunk_z: i32) -> Arc<ChunkShape> {
        self.carved.get((chunk_x, chunk_z), || {
            let (mut shape, tops) = self.surfaced(chunk_x, chunk_z);
            self.generated_tops.get((chunk_x, chunk_z), || tops);
            self.carve(&mut shape, chunk_x, chunk_z);
            shape.share();
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

    /// The liquid ticks the generated chunk's queue holds, for the game to run: (cell, lava), in order (`Features::game_ticks`).
    pub fn chunk_game_ticks(&self, chunk_x: i32, chunk_z: i32) -> Vec<([i32; 3], bool)> {
        self.with_inputs(|inputs| self.features.game_ticks((chunk_x, chunk_z), inputs))
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

    /// `Features::canonical_shuffles`, for the oracle test's `SAPLING_ORACLE_CANONICAL_SHUFFLES`.
    #[doc(hidden)]
    pub fn shuffle_canonically(&mut self) {
        self.features.canonical_shuffles = true;
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

    /// Process-wide: the dead coral (a `CoralColor`, 0..5) the coral crust templates place; see feature/coral/crust.rs.
    #[doc(hidden)]
    pub fn set_dead_coral(color: usize) {
        feature::set_dead_coral(color);
    }

    /// Molang's `query.noise(x, z)` (examples/query_noise.rs).
    #[doc(hidden)]
    pub fn query_noise(x: f32, z: f32) -> f32 {
        feature::query_noise(x, z)
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
        let in_tunnels = |[x, y, z]: [i32; 3]| !self.structures.in_v2(&self.density, (x, y, z)) && self.structures.in_structure(&self.density, (x, y, z), false);
        let carved = |x, z| self.carved(x, z);
        let generated_tops = |x, z| self.generated_tops(x, z);
        let inputs = feature::Inputs { loaded: &loaded, carved: &carved, generated_tops: &generated_tops, biomes: &biomes, surface: &surface,in_structure: &in_structure, in_tunnels: &in_tunnels, island: None };
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

    /// Each structure start map's entries kept and made.
    #[doc(hidden)]
    pub fn structure_cache_counts(&self) -> Vec<(&'static str, usize, usize)> {
        self.structures.cache_counts()
    }

    /// Keeps only `starts` structure starts of each kind, so that they are dropped and built again
    /// (examples/walk.rs).
    #[doc(hidden)]
    pub fn with_structure_start_limit(mut self, areas: usize, starts: usize) -> Self {
        self.structures = structure::Structures::with_start_limit(self.seed, areas.max(1), starts);
        self
    }

    pub fn density(&self) -> &Density {
        &self.density
    }
}
