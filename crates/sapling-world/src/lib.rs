//! Worlds as a server holds them: chunk columns built with acacia-world, served in sub-chunk
//! request mode from a per-section encode cache. Templates, arenas and storage (docs/DESIGN.md
//! "Arenas", "Storage") come with milestone step 2; this is the in-memory world they will back.

mod column;
mod world;

pub use acacia_world::BlockRegistry;
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
