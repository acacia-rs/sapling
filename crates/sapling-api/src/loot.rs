//! What `/loot` reads and makes: a container's slots, a block's drop for a tool, and items
//! tossed into the world.

use crate::{BlockPos, Item, ShardCtx};

impl ShardCtx<'_> {
    /// The item in slot `slot` of the container block at `pos`; `None` for an empty slot or no
    /// container.
    pub fn container_item(&mut self, pos: BlockPos, slot: usize) -> Option<Item> {
        self.backend.container_item(pos, slot)
    }

    /// What the block at `pos` would leave if `tool` broke it (a bare hand where `None`); the
    /// block stays. Nothing where it is not loaded.
    pub fn block_loot(&mut self, pos: BlockPos, tool: Option<&Item>) -> Vec<Item> {
        self.backend.block_loot(pos, tool)
    }

    /// Tosses `item` up from `at`, the middle of the item, as BDS tosses each item `/loot spawn`
    /// makes. False where the world is not loaded or the game has no such item.
    pub fn drop_item(&mut self, item: &Item, at: [f32; 3]) -> bool {
        self.backend.drop_item(item, at)
    }
}
