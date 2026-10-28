//! Where a world's columns come from. See docs/DESIGN.md "Storage".

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use acacia_world::Chunk;
use sapling_leveldb::{ChunkKey, LevelDat, LevelDbWorld, PlayerData};
use sapling_worldgen::{Block, ChunkShape, PlaceFailure};

use crate::ChunkPos;
pub use crate::level::Level;
use crate::level::{read_rules, read_structures, write_rules};

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
}

impl StoredColumn {
    pub fn of(chunk: Chunk) -> Self {
        Self { chunk, block_entities: Vec::new(), actors: Vec::new(), pending_ticks: Vec::new() }
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
    /// The blocks the generator's structure `name` places when it starts in chunk `start`, over
    /// the shapes `load` gives; `None` where storage generates no structures.
    fn structure_writes(&self, _name: &str, _start: ChunkPos, _load: &ShapeLoader<'_>) -> Option<Result<Vec<([i32; 3], Block)>, PlaceFailure>> {
        None
    }
    /// [`WorldStorage::structure_writes`] for the jigsaw pool `pool`, as `/place jigsaw` assembles it.
    fn jigsaw_writes(&self, _pool: &str, _target: &str, _max_depth: u32, _at: [i32; 3], _load: &ShapeLoader<'_>) -> Option<Result<Vec<([i32; 3], Block)>, PlaceFailure>> {
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
    fn locate_stronghold(&self, x: i32, z: i32) -> Option<[i32; 3]> {
        (**self).locate_stronghold(x, z)
    }
    fn locate_structure(&self, name: &str, x: i32, z: i32) -> Option<[i32; 2]> {
        (**self).locate_structure(name, x, z)
    }
    fn generated_biome(&self, x: i32, y: i32, z: i32) -> Option<u32> {
        (**self).generated_biome(x, y, z)
    }
    fn structure_writes(&self, name: &str, start: ChunkPos, load: &ShapeLoader<'_>) -> Option<Result<Vec<([i32; 3], Block)>, PlaceFailure>> {
        (**self).structure_writes(name, start, load)
    }
    fn jigsaw_writes(&self, pool: &str, target: &str, max_depth: u32, at: [i32; 3], load: &ShapeLoader<'_>) -> Option<Result<Vec<([i32; 3], Block)>, PlaceFailure>> {
        (**self).jigsaw_writes(pool, target, max_depth, at, load)
    }
}

/// Columns kept in memory: empty, it is the void.
#[derive(Default)]
pub struct MemoryStorage {
    columns: Mutex<HashMap<(i32, ChunkPos), StoredColumn>>,
    players: Mutex<HashMap<String, PlayerData>>,
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

    fn load_player(&self, id: &str) -> Result<Option<PlayerData>, StorageError> {
        Ok(self.players.lock().expect("memory storage lock").get(id).cloned())
    }

    fn store_player(&self, id: &str, player: &PlayerData) -> Result<(), StorageError> {
        self.players.lock().expect("memory storage lock").insert(id.to_string(), player.clone());
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
        let portals = self.0.load_portals().unwrap_or_else(|error| {
            tracing::warn!(%error, "the portal records are unreadable; portals find none");
            Vec::new()
        });
        let dragon_fight = self.0.load_dragon_fight().unwrap_or_else(|error| {
            tracing::warn!(%error, "the dragon fight is unreadable; a new one begins");
            None
        });
        let scoreboard = self.0.load_scoreboard().unwrap_or_else(|error| {
            tracing::warn!(%error, "the scoreboard is unreadable; it starts empty");
            None
        });
        let [folder_structures, structures] = read_structures(&self.0);
        let ticking_areas = self.0.load_ticking_areas().unwrap_or_else(|error| {
            tracing::warn!(%error, "the ticking areas are unreadable; there are none");
            Vec::new()
        });
        let (spawn, rules, seed, hardcore) = (dat.spawn(), read_rules(dat), dat.seed().unwrap_or(0), dat.hardcore());
        let (structures, folder_structures) = (structures.into(), folder_structures.into());
        let (time, weather, difficulty) = (dat.time().unwrap_or(Level::default().time), dat.weather().unwrap_or(Level::default().weather), dat.difficulty());
        Level { spawn, time, rules, portals, seed, dragon_fight, scoreboard, structures, folder_structures, ticking_areas, hardcore, weather, difficulty, chunk_tick_range: None }
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
        Ok(column.map(|c| StoredColumn { chunk: c.chunk, block_entities: c.block_entities, actors: c.actors, pending_ticks: c.pending_ticks }))
    }

    fn store_column(&self, pos: ChunkPos, dimension: i32, column: &StoredColumn) -> Result<(), StorageError> {
        Ok(self.0.store_column(ChunkKey::new(pos.x, pos.z, dimension), &column.chunk, &column.block_entities, &column.actors, &column.pending_ticks)?)
    }

    fn load_player(&self, id: &str) -> Result<Option<PlayerData>, StorageError> {
        Ok(self.0.load_player(id)?)
    }

    fn store_player(&self, id: &str, player: &PlayerData) -> Result<(), StorageError> {
        Ok(self.0.store_player(id, player)?)
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
        dat.set_weather(level.weather);
        if let Some(difficulty) = level.difficulty {
            dat.set_difficulty(difficulty);
        }
        if let Some(chunks) = level.chunk_tick_range {
            dat.set_chunk_tick_range(chunks);
        }
        write_rules(&mut dat, &level.rules);
        if let Some(spawn) = level.spawn {
            dat.set_spawn(spawn);
        }
        self.0.store_portals(&level.portals)?;
        if let Some(board) = &level.scoreboard {
            self.0.store_scoreboard(board)?;
        }
        if let Some(fight) = &level.dragon_fight {
            self.0.store_dragon_fight(fight)?;
        }
        let structures: Vec<(String, Vec<u8>)> = level.structures.iter().map(|(name, bytes)| (name.clone(), bytes.clone())).collect();
        self.0.store_structures(&structures)?;
        self.0.store_ticking_areas(&level.ticking_areas)?;
        Ok(self.0.flush(&dat)?)
    }

    fn sync(&self) -> Result<(), StorageError> {
        Ok(self.0.db().flush()?)
    }
}
