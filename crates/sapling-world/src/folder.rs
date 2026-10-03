//! Vanilla world folders as worlds.

use std::path::Path;
use std::sync::Arc;

use crate::storage::LevelDbStorage;
use crate::{ChunkPos, World};

/// `SpawnY` of a world whose spawn was never set by hand: "on the surface".
const SURFACE: i32 = i16::MAX as i32;
/// Where to stand when the spawn column is empty.
const VOID_SPAWN_Y: i32 = 64;

/// A world opened from a folder, with what its `level.dat` says about it.
pub struct WorldFolder {
    pub world: World,
    pub name: Option<String>,
    /// The saved spawn as a feet position, on the surface when the save leaves y open.
    pub spawn: [f32; 3],
}

impl World {
    /// Opens the overworld of a vanilla world folder. Blocks on disk for `level.dat`, the
    /// database log and the spawn column.
    pub fn open_folder(folder: &Path) -> Result<WorldFolder, sapling_leveldb::Error> {
        let storage = Arc::new(LevelDbStorage::open(folder)?);
        let (name, saved) = (storage.level().name().map(str::to_owned), storage.level().spawn().unwrap_or([0, SURFACE, 0]));
        let mut world = World::with_storage(storage);
        let spawn = world.spawn_at(saved);
        Ok(WorldFolder { world, name, spawn })
    }

    /// The feet position for a saved spawn block: centred on it, and when its y is not a real
    /// height, on top of the surface there. Loads the spawn column on the calling thread.
    pub fn spawn_at(&mut self, [x, y, z]: [i32; 3]) -> [f32; 3] {
        self.load_now(ChunkPos::of_block(x, z));
        let y = if y == SURFACE { self.surface(x, z).map_or(VOID_SPAWN_Y, |top| top + 1) } else { y };
        [x as f32 + 0.5, y as f32, z as f32 + 0.5]
    }
}
