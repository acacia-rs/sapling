//! Vanilla world folders as worlds.

use std::path::Path;
use std::sync::Arc;

use crate::generated::Generated;
use crate::storage::{Level, LevelDbStorage, WorldStorage};
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
    storage: Arc<LevelDbStorage>,
}

impl WorldFolder {
    /// Loads the spawn column on the calling thread.
    fn of(storage: LevelDbStorage) -> Self {
        let storage = Arc::new(storage);
        let dat = storage.level_dat();
        let (name, game_type, difficulty) = (dat.name().map(str::to_owned), dat.game_type(), dat.difficulty());
        let (world, spawn) = World::with_level(storage.clone(), storage.level());
        Self { world, name, spawn, game_type, difficulty, storage }
    }

    /// The same folder with every overworld column it lacks generated from its `level.dat` seed,
    /// as the game fills the chunks it never saved ([`Generated`]). Reopens the world, so the
    /// spawn column is loaded again on the calling thread. Without a seed the holes stay void.
    pub fn generate_missing(self) -> Self {
        let Some(seed) = self.storage.level_dat().seed() else {
            tracing::warn!("level.dat has no RandomSeed; columns the folder lacks stay void");
            return self;
        };
        let level = self.world.level().clone();
        let (world, spawn) = World::with_level(Arc::new(Generated::new(self.storage.clone(), seed)), level);
        Self { world, spawn, ..self }
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

    /// The world of `storage` with `level`, and its spawn as [`World::load_spawn`] finds it.
    fn with_level(storage: Arc<dyn WorldStorage>, level: Level) -> (World, [f32; 3]) {
        let mut world = World::with_storage(storage);
        world.level = level;
        let spawn = world.load_spawn();
        (world, spawn)
    }

    /// The feet position of the world's spawn ([`Level::spawn`]; without one, the surface above
    /// block (0, 0)). Loads the spawn column on the calling thread.
    pub fn load_spawn(&mut self) -> [f32; 3] {
        self.spawn_at(self.level.spawn.unwrap_or([0, SURFACE, 0]))
    }

    /// The feet position for a spawn block: centred on it, and when its y is not a real height,
    /// on top of the surface there. Loads the spawn column on the calling thread.
    pub(crate) fn spawn_at(&mut self, [x, y, z]: [i32; 3]) -> [f32; 3] {
        self.load_now(ChunkPos::of_block(x, z));
        let y = if y == SURFACE { self.surface(x, z).map_or(VOID_SPAWN_Y, |top| top + 1) } else { y };
        [x as f32 + 0.5, y as f32, z as f32 + 0.5]
    }
}
