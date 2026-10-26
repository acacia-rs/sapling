//! Vanilla Bedrock world folders: the LevelDB store with Mojang's compressions, the chunk key
//! layout, `level.dat`, and the disk chunk codec, decoding into acacia-world chunks. Formats and
//! sources: research/worlds-and-ecs.md §1.

mod actor_harm;
mod actor_raising;
mod actor_riding;
mod actor_tags;
mod actors;
mod block_entities;
mod container;
mod data3d;
mod db;
mod dragon_fight;
mod item;
mod keys;
mod level_dat;
mod player;
mod portals;
mod records;
mod scoreboard;
mod states;
mod structure;
mod sub_chunk;
mod ticking_areas;
mod world;

use std::path::PathBuf;

pub use actor_harm::harmed_actor;
pub use actor_riding::{actor_chest, actor_links, chested_actor, flagged_actor, linked_actor, varied_actor};
pub use actor_tags::{actor_tags, tagged_actor};
pub use scoreboard::{DisplayRecord, ObjectiveRecord, ScoreIdentity, ScoreboardRecord, decode_scoreboard, encode_scoreboard};
pub use actor_raising::{Raising, raised_actor, raising, regrouped_actor};
pub use actors::{Actor, Painting, State, actors, falling_block, lying_stack, moved_actor, new_actor, new_falling_block_actor, new_item_actor, new_orb_actor, new_shot_actor, orb_value, orb_worth, placed_actor, shot_record, stored_actors, ShotRecord};
pub use item::ItemStack;
pub use player::PlayerData;
pub use portals::{PortalRecord, decode_portals, encode_portals};
pub use dragon_fight::{DragonFightRecord, decode_dragon_fight, encode_dragon_fight};
pub use block_entities::{BlockEntity, block_entities, with_block_entity, without_block_entity};
pub use container::{container_items, new_block_entity, with_container_items};
pub use data3d::Data3d;
pub use db::{Batch, Db};
pub use keys::{ChunkKey, ParsedKey, Tag};
pub use level_dat::LevelDat;
pub use states::{BlockStates, PLACEHOLDER, block_of_hash, palette_block};
pub use structure::{STRUCTURE_PREFIX, StructureTemplate, structure_name};
pub use sub_chunk::{Layer, SubChunk};
pub use ticking_areas::{TICKING_AREA_PREFIX, TickingAreaRecord};
pub use world::{DiskColumn, LevelDbWorld};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}: {1}")]
    Io(PathBuf, #[source] std::io::Error),
    #[error("leveldb: {0}")]
    Db(#[from] rusty_leveldb::Status),
    #[error("level.dat: {0}")]
    LevelDat(&'static str),
    #[error("unsupported sub-chunk version {0}")]
    SubChunkVersion(u8),
    #[error("sub-chunk: {0}")]
    SubChunk(&'static str),
    #[error("player: {0}")]
    Player(&'static str),
    #[error("actors: {0}")]
    Actor(&'static str),
    #[error("block state {0} cannot be written")]
    State(String),
    #[error("biomes: {0}")]
    Biomes(&'static str),
    #[error("structure template: {0}")]
    Structure(&'static str),
    #[error("ticking area: {0}")]
    TickingArea(&'static str),
    #[error(transparent)]
    Decode(#[from] acacia_proto::DecodeError),
}
