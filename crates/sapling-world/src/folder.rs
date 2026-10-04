//! Vanilla world folders as worlds.

use std::path::Path;
use std::sync::Arc;

use crate::storage::LevelDbStorage;
use crate::{ChunkPos, World};

/// `SpawnY` of a world whose spawn was never set by hand: "on the surface".
const SURFACE: i32 = i16::MAX as i32;
/// Where to stand when the spawn column is empty.
const VOID_SPAWN_Y: i32 = 64;

/// A world opened from a folder, with what its `level.dat` says about it. The spawn block and
/// the time are the world's own ([`World::level`]).
pub struct WorldFolder {
    pub world: World,
    pub name: Option<String>,
    /// The saved spawn as a feet position, on the surface when the save leaves y open.
    pub spawn: [f32; 3],
    /// The default game mode as `level.dat` numbers it: 0 survival, 1 creative, 2 adventure,
    /// 6 spectator.
    pub game_type: Option<i32>,
    /// 0 peaceful to 3 hard.
    pub difficulty: Option<i32>,
}

impl WorldFolder {
    /// Loads the spawn column on the calling thread.
    fn of(storage: LevelDbStorage) -> Self {
        let storage = Arc::new(storage);
        let (dat, level) = (storage.level_dat(), storage.level());
        let (name, game_type, difficulty) = (dat.name().map(str::to_owned), dat.game_type(), dat.difficulty());
        let mut world = World::with_storage(storage);
        *world.level_mut() = level;
        let spawn = world.spawn_at(level.spawn.unwrap_or([0, SURFACE, 0]));
        Self { world, name, spawn, game_type, difficulty }
    }
}

impl World {
    /// Opens the overworld of a vanilla world folder in place; what is saved goes to the folder.
    /// Blocks on disk for `level.dat`, the database log and the spawn column.
    pub fn open_folder(folder: &Path) -> Result<WorldFolder, sapling_leveldb::Error> {
        LevelDbStorage::open(folder).map(WorldFolder::of)
    }

    /// The same from a copy of the folder held in memory: the folder is only read, and what is
    /// saved is lost with the world.
    pub fn open_folder_in_memory(folder: &Path) -> Result<WorldFolder, sapling_leveldb::Error> {
        LevelDbStorage::open_in_memory(folder).map(WorldFolder::of)
    }

    /// The feet position for a spawn block: centred on it, and when its y is not a real height,
    /// on top of the surface there. Loads the spawn column on the calling thread.
    pub(crate) fn spawn_at(&mut self, [x, y, z]: [i32; 3]) -> [f32; 3] {
        self.load_now(ChunkPos::of_block(x, z));
        let y = if y == SURFACE { self.surface(x, z).map_or(VOID_SPAWN_Y, |top| top + 1) } else { y };
        [x as f32 + 0.5, y as f32, z as f32 + 0.5]
    }
}
