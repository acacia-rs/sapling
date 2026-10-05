//! Worlds as a server holds them: chunk columns built with acacia-world, loaded from a
//! [`WorldStorage`] on I/O threads and served in sub-chunk request mode from a per-column encode
//! cache that every player shares. A [`Template`] is a world frozen in memory that arenas
//! (`World::arena`) read until they write (docs/DESIGN.md "Arenas").

mod column;
mod folder;
mod loader;
mod players;
mod storage;
mod template;
#[cfg(test)]
mod tests;
mod tiles;
mod world;

pub use acacia_proto::types::Blob;
pub use acacia_world::{BlockRegistry, Chunk, Dimension, LevelChunkData, Material, Mining, Tool, ToolKind, ToolTier};
pub use column::{SubChunkAnswer, empty_column};
pub use folder::WorldFolder;
pub use sapling_leveldb::LevelDat;
pub use players::PlayerLoad;
pub use sapling_leveldb::{
    Actor, BlockEntity, ItemStack, Painting, PlayerData, State as ActorState, actors, container_items, new_block_entity, with_container_items,
};
pub use storage::{Level, LevelDbStorage, MemoryStorage, StorageError, StoredColumn, WorldStorage};
pub use template::Template;
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
