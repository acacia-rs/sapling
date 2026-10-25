//! Worlds as a server holds them: chunk columns built with acacia-world, loaded from a
//! [`WorldStorage`] on I/O threads and served in sub-chunk request mode from a per-column encode
//! cache that every player shares. A [`Template`] is a world frozen in memory that arenas
//! (`World::arena`) read until they write (docs/DESIGN.md "Arenas").

mod column;
#[cfg(test)]
mod dimension_tests;
mod folder;
mod generated;
mod light;
mod load_queue;
mod loader;
mod players;
mod storage;
mod template;
#[cfg(test)]
mod tests;
mod tiles;
mod world;
mod world_io;

pub use acacia_proto::types::Blob;
pub use acacia_world::{Aabb, BlockRegistry, Chunk, Dimension, LevelChunkData, Material, Mining, Tool, ToolKind, ToolTier};
pub use column::{SubChunkAnswer, empty_column};
pub use folder::WorldFolder;
pub use generated::Generated;
pub use light::FULL_LIGHT;
pub use sapling_leveldb::LevelDat;
pub use players::PlayerLoad;
pub use sapling_leveldb::{
    DragonFightRecord, PortalRecord, DisplayRecord, ObjectiveRecord, ScoreIdentity, ScoreboardRecord, actor_tags, decode_scoreboard, encode_scoreboard, tagged_actor,
    Actor, BlockEntity, ItemStack, Painting, PlayerData, State as ActorState, actors, block_of_hash, container_items, falling_block, harmed_actor, moved_actor, new_actor, new_falling_block_actor, new_item_actor, new_orb_actor, orb_value, orb_worth, new_block_entity, stored_actors,
    with_container_items,
};
pub use storage::{Level, LevelDbStorage, MemoryStorage, StorageError, StoredColumn, WorldStorage};
pub use template::Template;
pub use world::{END, NETHER, OVERWORLD, World};

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
