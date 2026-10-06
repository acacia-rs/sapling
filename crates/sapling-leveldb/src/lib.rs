//! Vanilla Bedrock world folders: the LevelDB store with Mojang's compressions, the chunk key
//! layout, `level.dat`, and the disk chunk codec, decoding into acacia-world chunks. Formats and
//! sources: research/worlds-and-ecs.md §1.

mod actors;
mod block_entities;
mod container;
mod data3d;
mod db;
mod item;
mod keys;
mod level_dat;
mod player;
mod records;
mod states;
mod sub_chunk;
mod world;

use std::path::PathBuf;

pub use actors::{Actor, Painting, State, actors, moved_actor, new_actor, stored_actors};
pub use item::ItemStack;
pub use player::PlayerData;
pub use block_entities::{BlockEntity, block_entities, with_block_entity, without_block_entity};
pub use container::{container_items, new_block_entity, with_container_items};
pub use data3d::Data3d;
pub use db::{Batch, Db};
pub use keys::{ChunkKey, ParsedKey, Tag};
pub use level_dat::LevelDat;
pub use states::{BlockStates, PLACEHOLDER, block_of_hash};
pub use sub_chunk::{Layer, SubChunk};
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
    #[error(transparent)]
    Decode(#[from] acacia_proto::DecodeError),
}
