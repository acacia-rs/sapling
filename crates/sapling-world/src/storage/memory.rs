use std::collections::HashMap;
use std::sync::Mutex;

use acacia_world::Chunk;
use sapling_leveldb::PlayerData;

use super::{StorageError, StoredColumn, WorldStorage};
use crate::ChunkPos;

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
