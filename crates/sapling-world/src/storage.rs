//! Where a world's columns come from. See docs/DESIGN.md "Storage".

mod leveldb;
mod memory;

use acacia_world::Chunk;
use sapling_leveldb::PlayerData;
use sapling_worldgen::{ChunkShape, PlaceFailure, StructureWrites, TemplateActor};

use crate::ChunkPos;
pub use crate::level::Level;
pub use leveldb::LevelDbStorage;
pub use memory::MemoryStorage;

pub type StorageError = Box<dyn std::error::Error + Send + Sync>;

/// A column as storage holds it.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredColumn {
    pub chunk: Chunk,
    /// Concatenated little-endian NBT compounds, each naming its block in `x`, `y`, `z`.
    pub block_entities: Vec<u8>,
    /// The column's mobs, items and the like, in the same form (`sapling_leveldb::actors`).
    pub actors: Vec<u8>,
    /// The block ticks waiting in it, as one compound (`sapling_leveldb::pending_ticks`).
    pub pending_ticks: Vec<u8>,
    /// Its structures' spawn areas, one record (`sapling_leveldb::decode_spawn_areas`).
    pub spawn_areas: Vec<u8>,
    /// Mobs its generation placed and nothing has made yet: the shard makes them as the column
    /// arrives, and storage never keeps them.
    pub template_actors: Vec<TemplateActor>,
}

impl StoredColumn {
    pub fn of(chunk: Chunk) -> Self {
        Self { chunk, block_entities: Vec::new(), actors: Vec::new(), pending_ticks: Vec::new(), spawn_areas: Vec::new(), template_actors: Vec::new() }
    }
}

/// A world's persistent side. Calls block on disk, so they are made from the world's I/O
/// threads (loader.rs) and from startup code, never from a shard's tick.
pub trait WorldStorage: Send + Sync + 'static {
    /// How many I/O threads a world gives this storage: the loads worth running at once.
    fn load_threads(&self) -> usize {
        2
    }
    /// The column at `pos`; `None` when storage has none there, which the world shows as void.
    fn load_column(&self, pos: ChunkPos, dimension: i32) -> Result<Option<StoredColumn>, StorageError>;
    /// Saves the column at `pos`; what else storage holds there is kept.
    fn store_column(&self, pos: ChunkPos, dimension: i32, column: &StoredColumn) -> Result<(), StorageError>;
    /// What was kept of the player with this id (its UUID as text); `None` for a first visit.
    fn load_player(&self, _id: &str) -> Result<Option<PlayerData>, StorageError> {
        Ok(None)
    }
    /// Keeps a player's items for their next visit.
    fn store_player(&self, _id: &str, _player: &PlayerData) -> Result<(), StorageError> {
        Ok(())
    }
    /// Every column storage holds in a dimension, for loading a world whole (`World::load_all`).
    fn columns(&self, _dimension: i32) -> Result<Vec<ChunkPos>, StorageError> {
        Ok(Vec::new())
    }
    /// Saves `level` and makes what was stored durable. Called once, when the world closes.
    fn flush(&self, _level: &Level) -> Result<(), StorageError> {
        Ok(())
    }
    /// Makes what was stored durable and leaves the level alone: what a world of another
    /// dimension than the overworld calls when it closes.
    fn sync(&self) -> Result<(), StorageError> {
        Ok(())
    }
    /// The folder the world's files are in; `None` for storage without one.
    fn folder(&self) -> Option<std::path::PathBuf> {
        None
    }
    /// The block an eye of ender thrown from (x, z) flies to; `None` where storage makes no
    /// strongholds. Computes without disk, so a tick may call it.
    fn locate_stronghold(&self, _x: i32, _z: i32) -> Option<[i32; 3]> {
        None
    }
    /// The block column (x, z) `/locate structure` answers for the structure `name` from block
    /// (x, z); `None` where storage makes none. Computes without disk.
    fn locate_structure(&self, _name: &str, _x: i32, _z: i32) -> Option<[i32; 2]> {
        None
    }
    /// The biome the generator gives the cell; `None` where storage generates nothing.
    fn generated_biome(&self, _x: i32, _y: i32, _z: i32) -> Option<u32> {
        None
    }
    /// One above the highest block the generator's noise shape puts in column (x, z); `None` where storage
    /// generates nothing.
    fn generated_height(&self, _x: i32, _z: i32) -> Option<i32> {
        None
    }
    /// The blocks the generator's structure `name` places when `/place structure` starts it at the
    /// block `at`, over the shapes `load` gives, its start height kept with `keep_height`
    /// (`ignoreStartHeight`); `None` where storage generates no structures.
    fn structure_writes(&self, _name: &str, _at: [i32; 3], _keep_height: bool, _load: &ShapeLoader<'_>) -> Option<Result<StructureWrites, PlaceFailure>> {
        None
    }
    /// [`WorldStorage::structure_writes`] for the jigsaw pool `pool`, as `/place jigsaw` assembles it.
    fn jigsaw_writes(&self, _pool: &str, _target: &str, _max_depth: u32, _at: [i32; 3], _load: &ShapeLoader<'_>) -> Option<Result<StructureWrites, PlaceFailure>> {
        None
    }
}

/// What the generator draws a structure over: the shape of chunk (x, z) as the world holds it.
pub type ShapeLoader<'a> = dyn Fn(i32, i32) -> std::sync::Arc<ChunkShape> + 'a;

/// Shared storage, e.g. a folder's database wrapped in [`crate::Generated`] while still held.
impl<S: WorldStorage + ?Sized> WorldStorage for std::sync::Arc<S> {
    fn load_threads(&self) -> usize {
        (**self).load_threads()
    }
    fn load_column(&self, pos: ChunkPos, dimension: i32) -> Result<Option<StoredColumn>, StorageError> {
        (**self).load_column(pos, dimension)
    }
    fn store_column(&self, pos: ChunkPos, dimension: i32, column: &StoredColumn) -> Result<(), StorageError> {
        (**self).store_column(pos, dimension, column)
    }
    fn load_player(&self, id: &str) -> Result<Option<PlayerData>, StorageError> {
        (**self).load_player(id)
    }
    fn store_player(&self, id: &str, player: &PlayerData) -> Result<(), StorageError> {
        (**self).store_player(id, player)
    }
    fn columns(&self, dimension: i32) -> Result<Vec<ChunkPos>, StorageError> {
        (**self).columns(dimension)
    }
    fn flush(&self, level: &Level) -> Result<(), StorageError> {
        (**self).flush(level)
    }
    fn sync(&self) -> Result<(), StorageError> {
        (**self).sync()
    }
    fn folder(&self) -> Option<std::path::PathBuf> {
        (**self).folder()
    }
    fn locate_stronghold(&self, x: i32, z: i32) -> Option<[i32; 3]> {
        (**self).locate_stronghold(x, z)
    }
    fn locate_structure(&self, name: &str, x: i32, z: i32) -> Option<[i32; 2]> {
        (**self).locate_structure(name, x, z)
    }
    fn generated_biome(&self, x: i32, y: i32, z: i32) -> Option<u32> {
        (**self).generated_biome(x, y, z)
    }
    fn generated_height(&self, x: i32, z: i32) -> Option<i32> {
        (**self).generated_height(x, z)
    }
    fn structure_writes(&self, name: &str, at: [i32; 3], keep_height: bool, load: &ShapeLoader<'_>) -> Option<Result<StructureWrites, PlaceFailure>> {
        (**self).structure_writes(name, at, keep_height, load)
    }
    fn jigsaw_writes(&self, pool: &str, target: &str, max_depth: u32, at: [i32; 3], load: &ShapeLoader<'_>) -> Option<Result<StructureWrites, PlaceFailure>> {
        (**self).jigsaw_writes(pool, target, max_depth, at, load)
    }
}
