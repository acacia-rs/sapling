//! Where a world's columns come from. See docs/DESIGN.md "Storage".

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use acacia_world::Chunk;
use sapling_leveldb::{ChunkKey, LevelDat, LevelDbWorld};

use crate::ChunkPos;

pub type StorageError = Box<dyn std::error::Error + Send + Sync>;

/// A column as storage holds it.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredColumn {
    pub chunk: Chunk,
    /// Concatenated little-endian NBT compounds, each naming its block in `x`, `y`, `z`.
    pub block_entities: Vec<u8>,
    /// The column's mobs, items and the like, in the same form (`sapling_leveldb::actors`).
    pub actors: Vec<u8>,
}

impl StoredColumn {
    pub fn of(chunk: Chunk) -> Self {
        Self { chunk, block_entities: Vec::new(), actors: Vec::new() }
    }
}

/// What a world keeps besides its columns; storage saves it when the world closes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Level {
    /// The spawn block; y 32767 leaves the height to the surface, as vanilla saves it.
    pub spawn: Option<[i32; 3]>,
    /// The time of day counter: a day is 24000 long and the count runs on past it.
    pub time: i64,
    /// Whether `time` advances.
    pub daylight_cycle: bool,
}

impl Default for Level {
    /// Noon, since a world without storage has nothing to say when it is.
    fn default() -> Self {
        Self { spawn: None, time: 6000, daylight_cycle: true }
    }
}

/// A world's persistent side. Calls block on disk, so they are made from the world's I/O
/// threads (loader.rs) and from startup code, never from a shard's tick.
pub trait WorldStorage: Send + Sync + 'static {
    /// The column at `pos`; `None` when storage has none there, which the world shows as void.
    fn load_column(&self, pos: ChunkPos, dimension: i32) -> Result<Option<StoredColumn>, StorageError>;
    /// Saves the column at `pos`; what else storage holds there is kept.
    fn store_column(&self, pos: ChunkPos, dimension: i32, column: &StoredColumn) -> Result<(), StorageError>;
    /// Every column storage holds in a dimension, for loading a world whole (`World::load_all`).
    fn columns(&self, _dimension: i32) -> Result<Vec<ChunkPos>, StorageError> {
        Ok(Vec::new())
    }
    /// Saves `level` and makes what was stored durable. Called once, when the world closes.
    fn flush(&self, _level: &Level) -> Result<(), StorageError> {
        Ok(())
    }
}

/// Columns kept in memory: empty, it is the void.
#[derive(Default)]
pub struct MemoryStorage {
    columns: Mutex<HashMap<(i32, ChunkPos), StoredColumn>>,
}

impl MemoryStorage {
    pub fn insert(&self, dimension: i32, chunk: Chunk) {
        let pos = ChunkPos::new(chunk.x, chunk.z);
        self.columns.lock().expect("memory storage lock").insert((dimension, pos), StoredColumn::of(chunk));
    }
}

impl WorldStorage for MemoryStorage {
    fn load_column(&self, pos: ChunkPos, dimension: i32) -> Result<Option<StoredColumn>, StorageError> {
        Ok(self.columns.lock().expect("memory storage lock").get(&(dimension, pos)).cloned())
    }

    fn store_column(&self, pos: ChunkPos, dimension: i32, column: &StoredColumn) -> Result<(), StorageError> {
        self.columns.lock().expect("memory storage lock").insert((dimension, pos), column.clone());
        Ok(())
    }

    fn columns(&self, dimension: i32) -> Result<Vec<ChunkPos>, StorageError> {
        let mut held: Vec<ChunkPos> = self.columns.lock().expect("memory storage lock").keys().filter(|(dim, _)| *dim == dimension).map(|(_, pos)| *pos).collect();
        held.sort();
        Ok(held)
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

    /// `level.dat` as it was when the folder was opened.
    pub fn level_dat(&self) -> &LevelDat {
        self.0.level()
    }

    /// What `level.dat` says of [`Level`]'s fields.
    pub fn level(&self) -> Level {
        let dat = self.0.level();
        Level { spawn: dat.spawn(), time: dat.time().unwrap_or(Level::default().time), daylight_cycle: dat.daylight_cycle() }
    }
}

/// The game version sapling writes worlds for, as `lastOpenedWithVersion` spells it.
fn game_version() -> [i32; 5] {
    let mut parts = acacia_proto::GAME_VERSION.split('.').map(|part| part.parse().unwrap_or(0));
    std::array::from_fn(|_| parts.next().unwrap_or(0))
}

impl WorldStorage for LevelDbStorage {
    fn load_column(&self, pos: ChunkPos, dimension: i32) -> Result<Option<StoredColumn>, StorageError> {
        let column = self.0.load_column(ChunkKey::new(pos.x, pos.z, dimension))?;
        Ok(column.map(|c| StoredColumn { chunk: c.chunk, block_entities: c.block_entities, actors: c.actors }))
    }

    fn store_column(&self, pos: ChunkPos, dimension: i32, column: &StoredColumn) -> Result<(), StorageError> {
        Ok(self.0.store_column(ChunkKey::new(pos.x, pos.z, dimension), &column.chunk, &column.block_entities, &column.actors)?)
    }

    fn columns(&self, dimension: i32) -> Result<Vec<ChunkPos>, StorageError> {
        Ok(self.0.columns()?.into_iter().filter(|key| key.dimension == dimension).map(|key| ChunkPos::new(key.x, key.z)).collect())
    }

    fn flush(&self, level: &Level) -> Result<(), StorageError> {
        let mut dat = self.0.level().clone();
        dat.set_last_played(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64);
        // The game's own stamp of this version carries a revision ours lacks: keep it.
        let version = game_version();
        if !dat.last_opened_with().is_some_and(|stamp| stamp.get(..3) == Some(&version[..3])) {
            dat.set_last_opened_with(version);
        }
        dat.set_time(level.time);
        if let Some(spawn) = level.spawn {
            dat.set_spawn(spawn);
        }
        Ok(self.0.flush(&dat)?)
    }
}
