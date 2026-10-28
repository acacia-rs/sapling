//! Storage in memory that remembers the level it was last flushed with, for tests across a restart.

use std::sync::{Arc, Mutex};

use sapling_world::{ChunkPos, Level, MemoryStorage, PlayerData, StorageError, StoredColumn, World, WorldStorage};

use super::{Harness, config};

pub struct Kept {
    pub columns: Arc<MemoryStorage>,
    pub level: Mutex<Option<Level>>,
}

impl Kept {
    pub fn new(columns: Arc<MemoryStorage>) -> Arc<Self> {
        Arc::new(Self { columns, level: Mutex::new(None) })
    }

    /// The level last flushed.
    pub fn level(&self) -> Option<Level> {
        self.level.lock().unwrap().clone()
    }
}

impl WorldStorage for Kept {
    fn load_column(&self, pos: ChunkPos, dimension: i32) -> Result<Option<StoredColumn>, StorageError> {
        self.columns.load_column(pos, dimension)
    }
    fn store_column(&self, pos: ChunkPos, dimension: i32, column: &StoredColumn) -> Result<(), StorageError> {
        self.columns.store_column(pos, dimension, column)
    }
    fn load_player(&self, id: &str) -> Result<Option<PlayerData>, StorageError> {
        self.columns.load_player(id)
    }
    fn store_player(&self, id: &str, player: &PlayerData) -> Result<(), StorageError> {
        self.columns.store_player(id, player)
    }
    fn flush(&self, level: &Level) -> Result<(), StorageError> {
        *self.level.lock().unwrap() = Some(level.clone());
        Ok(())
    }
}

/// A shard on `storage`, with the level it last flushed.
pub fn opened(storage: &Arc<Kept>) -> Harness {
    let mut world = World::with_storage(storage.clone());
    if let Some(level) = storage.level() {
        *world.level_mut() = level;
    }
    Harness::with(config(), world)
}
