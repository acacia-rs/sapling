//! Worlds as a server holds them: chunk columns built with acacia-world, loaded from a
//! [`WorldStorage`] on I/O threads and served in sub-chunk request mode from a per-column encode
//! cache that every player shares. A [`Template`] is a world frozen in memory that arenas
//! (`World::arena`) read until they write (docs/DESIGN.md "Arenas").

mod climate;
mod column;
#[cfg(test)]
mod dimension_tests;
mod folder;
mod generated;
mod level;
mod light;
mod load_queue;
mod loader;
mod locate;
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
pub use generated::{Generated, PlacedStructure};
pub use light::FULL_LIGHT;
pub use sapling_leveldb::{LevelDat, WeatherRecord};
pub use players::PlayerLoad;
pub use sapling_leveldb::{Raising, raised_actor, raising, regrouped_actor};
pub use sapling_leveldb::{actor_attributes, actor_pose, attributed_actor, geared_actor};
pub use sapling_leveldb::{WitherRecord, actor_effects, actor_fuse, effected_actor, fused_actor, wither_record, withered_actor};
pub use sapling_leveldb::{Occupant, hive_occupants, new_hive, occupied_hive};
pub use sapling_leveldb::{
    DragonFightRecord, PortalRecord, DisplayRecord, ObjectiveRecord, ScoreIdentity, ScoreboardRecord, actor_tags, decode_scoreboard, encode_scoreboard, tagged_actor,
    Actor, BlockEntity, ItemStack, Painting, PlayerData, RecipeRecord, State as ActorState, actors, block_of_hash, container_items, falling_block, harmed_actor, lying_stack, moved_actor, new_actor, new_falling_block_actor, new_item_actor, new_orb_actor, new_painting_actor, new_shot_actor, orb_value, orb_worth, new_block_entity, shot_record, stored_actors, ShotRecord,
    with_container_items, actor_chest, actor_links, chested_actor, flagged_actor, linked_actor, varied_actor,
    StructureTemplate, TickingAreaRecord, TrackerRecord, RaidRecord, MapRecord, structure_name, palette_block, placed_actor, PendingTick, decode_pending_ticks, encode_pending_ticks,
};
pub use storage::{Level, LevelDbStorage, MemoryStorage, ShapeLoader, StorageError, StoredColumn, WorldStorage};
pub use sapling_worldgen::{PlaceFailure, is_feature};
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
