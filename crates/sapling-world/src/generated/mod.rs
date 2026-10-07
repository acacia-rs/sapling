//! Storage whose holes are generated terrain. See docs/DESIGN.md "Storage".

mod convert;
#[cfg(test)]
mod tests;

use acacia_world::{BlockRegistry, Dimension};
use sapling_leveldb::PlayerData;
use sapling_worldgen::OverworldGenerator;

use crate::ChunkPos;
use crate::storage::{Level, StorageError, StoredColumn, WorldStorage};
use convert::{BlockIds, generated_chunk};

const OVERWORLD: i32 = 0;

/// `inner` with every overworld column it lacks generated from `seed`, as BDS would generate it.
/// Generated columns are not stored: like any loaded column, one is saved only once written to.
/// Other dimensions keep their holes (void).
pub struct Generated<S> {
    inner: S,
    generator: OverworldGenerator,
    ids: BlockIds,
    dim: Dimension,
}

impl<S: WorldStorage> Generated<S> {
    /// Builds the generator's noises: tens of milliseconds, once per world.
    pub fn new(inner: S, seed: i64) -> Self {
        let registry = BlockRegistry::vanilla();
        Self { inner, generator: OverworldGenerator::new(seed), ids: BlockIds::of(registry), dim: Dimension::overworld(registry.air_id()) }
    }

    pub fn inner(&self) -> &S {
        &self.inner
    }

    /// The overworld column the generator makes at `pos`, whatever `inner` holds there.
    pub fn generate_column(&self, pos: ChunkPos) -> StoredColumn {
        let shape = self.generator.chunk_blocks(pos.x, pos.z);
        let biomes = self.generator.chunk_biomes(pos.x, pos.z);
        StoredColumn::of(generated_chunk(pos.x, pos.z, &shape, &biomes, &self.ids, self.dim))
    }
}

impl<S: WorldStorage> WorldStorage for Generated<S> {
    fn load_column(&self, pos: ChunkPos, dimension: i32) -> Result<Option<StoredColumn>, StorageError> {
        match self.inner.load_column(pos, dimension)? {
            None if dimension == OVERWORLD => Ok(Some(self.generate_column(pos))),
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
}
