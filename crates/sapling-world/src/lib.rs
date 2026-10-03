//! Worlds as a server holds them: chunk columns built with acacia-world, loaded from a
//! [`WorldStorage`] on I/O threads and served in sub-chunk request mode from a per-column encode
//! cache that every player shares. Templates and arenas (docs/DESIGN.md "Arenas") come with
//! milestone step 5.

mod column;
mod folder;
mod loader;
mod storage;
#[cfg(test)]
mod tests;
mod world;

pub use acacia_proto::types::Blob;
pub use acacia_world::{BlockRegistry, Chunk, Dimension, LevelChunkData};
pub use column::SubChunkAnswer;
pub use folder::WorldFolder;
pub use sapling_leveldb::LevelDat;
pub use storage::{LevelDbStorage, MemoryStorage, StorageError, StoredColumn, WorldStorage};
pub use world::World;

/// Chunk palettes carry block-state hashes, not runtime ids; StartGame must say the same.
pub const NETWORK_IDS_ARE_HASHES: bool = true;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}

impl ChunkPos {
    pub const fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    /// The chunk holding block column `(x, z)`.
    pub const fn of_block(x: i32, z: i32) -> Self {
        Self { x: x >> 4, z: z >> 4 }
    }
}
