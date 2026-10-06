//! What blocks hold beyond their state (a chest's items, a sign's text), read and written
//! through the world: an arena reads its template's until it writes its own.

use acacia_proto::nbt::Value;
use sapling_leveldb::BlockEntity;

use crate::{ChunkPos, World};

impl World {
    /// What the block at a position holds beyond its state, if anything.
    pub fn block_entity(&self, pos: [i32; 3]) -> Option<&BlockEntity> {
        self.column(ChunkPos::of_block(pos[0], pos[2]))?.block_entity(pos)
    }

    /// What the blocks of the column at `pos` hold beyond their state, if it is loaded; an
    /// arena's template columns it never wrote to are left out.
    pub fn block_entities_in(&self, pos: ChunkPos) -> Vec<&BlockEntity> {
        self.columns.get(&pos).map(|column| column.block_entities().collect()).unwrap_or_default()
    }

    /// The columns held in memory.
    pub fn loaded(&self) -> impl Iterator<Item = ChunkPos> + '_ {
        self.columns.keys().copied()
    }

    /// Makes `entity` what the block at `pos` holds; false where the column is not loaded or its
    /// block entities cannot be read.
    pub fn set_block_entity(&mut self, pos: [i32; 3], entity: &Value) -> bool {
        self.column_mut(ChunkPos::of_block(pos[0], pos[2])).is_some_and(|column| column.set_block_entity(pos, entity))
    }
}
