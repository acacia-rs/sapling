//! Where a world's columns come from. See docs/DESIGN.md "Storage".

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use acacia_world::Chunk;
use sapling_leveldb::{ChunkKey, LevelDat, LevelDbWorld};

use crate::ChunkPos;

pub type StorageError = Box<dyn std::error::Error + Send + Sync>;

/// A column as storage holds it.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredColumn {
    pub chunk: Chunk,
    /// Concatenated little-endian NBT compounds, opaque to the world for now.
    pub block_entities: Vec<u8>,
}

/// A world's persistent side. Calls block on disk, so they are made from the world's I/O
/// threads (loader.rs) and from startup code, never from a shard's tick.
pub trait WorldStorage: Send + Sync + 'static {
    /// The column at `pos`; `None` when storage has none there, which the world shows as void.
    fn load_column(&self, pos: ChunkPos, dimension: i32) -> Result<Option<StoredColumn>, StorageError>;
    // TODO: store_column, one write batch per column, once worlds are saved.
}

/// Columns kept in memory: empty, it is the void.
#[derive(Default)]
pub struct MemoryStorage {
    columns: Mutex<HashMap<(i32, ChunkPos), StoredColumn>>,
}

impl MemoryStorage {
    pub fn insert(&self, dimension: i32, chunk: Chunk) {
        let pos = ChunkPos::new(chunk.x, chunk.z);
        self.columns.lock().expect("memory storage lock").insert((dimension, pos), StoredColumn { chunk, block_entities: Vec::new() });
    }
}

impl WorldStorage for MemoryStorage {
    fn load_column(&self, pos: ChunkPos, dimension: i32) -> Result<Option<StoredColumn>, StorageError> {
        Ok(self.columns.lock().expect("memory storage lock").get(&(dimension, pos)).cloned())
    }
}

/// A vanilla world folder.
pub struct LevelDbStorage(LevelDbWorld);

impl LevelDbStorage {
    /// Opens the folder's database in place (`sapling_leveldb::Db::open`).
    pub fn open(folder: &Path) -> Result<Self, sapling_leveldb::Error> {
        LevelDbWorld::open(folder).map(Self)
    }

    /// Serves the world from a copy in memory; the folder is only read.
    pub fn open_in_memory(folder: &Path) -> Result<Self, sapling_leveldb::Error> {
        LevelDbWorld::open_in_memory(folder).map(Self)
    }

    pub fn level(&self) -> &LevelDat {
        self.0.level()
    }
}

impl WorldStorage for LevelDbStorage {
    fn load_column(&self, pos: ChunkPos, dimension: i32) -> Result<Option<StoredColumn>, StorageError> {
        let column = self.0.load_column(ChunkKey::new(pos.x, pos.z, dimension))?;
        Ok(column.map(|c| StoredColumn { chunk: c.chunk, block_entities: c.block_entities }))
    }
}
