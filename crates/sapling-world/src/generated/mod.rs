//! Storage whose holes are generated terrain. See docs/DESIGN.md "Storage".

mod convert;
mod grow;
#[cfg(test)]
mod tests;

use acacia_world::{BlockRegistry, Dimension};
use sapling_leveldb::PlayerData;
use sapling_worldgen::{ChunkBiomes, ChunkShape, EndGenerator, NetherGenerator, OverworldGenerator};

use crate::storage::{Level, StorageError, StoredColumn, WorldStorage};
use crate::{ChunkPos, END, NETHER, OVERWORLD};
use convert::{BlockIds, generated_chunk};

/// The generator of one dimension.
enum Generator {
    Overworld(OverworldGenerator),
    Nether(NetherGenerator),
    End(EndGenerator),
}

impl Generator {
    fn dimension(&self) -> i32 {
        match self {
            Self::Overworld(_) => OVERWORLD,
            Self::Nether(_) => NETHER,
            Self::End(_) => END,
        }
    }

    fn chunk(&self, pos: ChunkPos) -> (ChunkShape, std::sync::Arc<ChunkBiomes>) {
        match self {
            Self::Overworld(generator) => (generator.chunk_blocks(pos.x, pos.z), generator.chunk_biomes(pos.x, pos.z)),
            Self::Nether(generator) => (generator.chunk_blocks(pos.x, pos.z), generator.chunk_biomes(pos.x, pos.z)),
            Self::End(generator) => (generator.chunk_blocks(pos.x, pos.z), generator.chunk_biomes(pos.x, pos.z)),
        }
    }
}

/// `inner` with every column of one dimension it lacks generated from `seed`, as BDS would
/// generate it. Generated columns are not stored: like any loaded column, one is saved only once
/// written to. Other dimensions keep their holes (void).
pub struct Generated<S> {
    inner: S,
    generator: Generator,
    areas: usize,
    ids: BlockIds,
    dim: Dimension,
}

/// Half the hardware threads, at most four: the rest are for the server's ticks.
fn default_areas() -> usize {
    std::thread::available_parallelism().map_or(1, |n| n.get() / 2).clamp(1, 4)
}

impl<S: WorldStorage> Generated<S> {
    /// [`Generated::with_areas`] with [`default_areas`].
    pub fn new(inner: S, seed: i64) -> Self {
        Self::with_areas(inner, seed, default_areas())
    }

    /// Generates the overworld on `areas` I/O threads, each keeping to its own part of the world
    /// (load_queue.rs), with caches for that many areas at once. Peak memory grows with `areas`,
    /// about 250 MB each (docs/research/worldgen-server-load.md).
    /// Builds the generator's noises: tens of milliseconds, once per world.
    pub fn with_areas(inner: S, seed: i64, areas: usize) -> Self {
        let areas = areas.max(1);
        Self::of(inner, Generator::Overworld(OverworldGenerator::with_areas(seed, areas)), areas)
    }

    /// The Nether of `seed` where `inner` has none, on `areas` I/O threads sharing one generator.
    pub fn nether(inner: S, seed: i64, areas: usize) -> Self {
        Self::of(inner, Generator::Nether(NetherGenerator::new(seed)), areas.max(1))
    }

    /// The End of `seed` where `inner` has none, as [`Generated::nether`].
    pub fn end(inner: S, seed: i64, areas: usize) -> Self {
        Self::of(inner, Generator::End(EndGenerator::new(seed)), areas.max(1))
    }

    fn of(inner: S, generator: Generator, areas: usize) -> Self {
        let registry = BlockRegistry::vanilla();
        let dim = Dimension::from_id(generator.dimension(), registry.air_id());
        Self { inner, generator, areas, ids: BlockIds::of(registry), dim }
    }

    pub fn inner(&self) -> &S {
        &self.inner
    }

    /// The dimension this storage generates.
    pub fn dimension(&self) -> i32 {
        self.generator.dimension()
    }

    /// The column the generator makes at `pos`, whatever `inner` holds there.
    pub fn generate_column(&self, pos: ChunkPos) -> StoredColumn {
        let (shape, biomes) = self.generator.chunk(pos);
        StoredColumn::of(generated_chunk(pos.x, pos.z, &shape, &biomes, &self.ids, self.dim))
    }
}

impl<S: WorldStorage> WorldStorage for Generated<S> {
    fn load_threads(&self) -> usize {
        self.areas
    }

    fn load_column(&self, pos: ChunkPos, dimension: i32) -> Result<Option<StoredColumn>, StorageError> {
        match self.inner.load_column(pos, dimension)? {
            None if dimension == self.dimension() => Ok(Some(self.generate_column(pos))),
            stored => Ok(stored),
        }
    }

    fn store_column(&self, pos: ChunkPos, dimension: i32, column: &StoredColumn) -> Result<(), StorageError> {
        self.inner.store_column(pos, dimension, column)
    }

    fn load_player(&self, id: &str) -> Result<Option<PlayerData>, StorageError> {
        self.inner.load_player(id)
    }

    fn store_player(&self, id: &str, player: &PlayerData) -> Result<(), StorageError> {
        self.inner.store_player(id, player)
    }

    fn columns(&self, dimension: i32) -> Result<Vec<ChunkPos>, StorageError> {
        self.inner.columns(dimension)
    }

    fn flush(&self, level: &Level) -> Result<(), StorageError> {
        self.inner.flush(level)
    }

    fn sync(&self) -> Result<(), StorageError> {
        self.inner.sync()
    }
}
