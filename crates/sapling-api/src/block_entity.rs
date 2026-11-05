//! What a block holds beyond its state (a chest's items, a sign's text), carried whole from one
//! place to another, as `/clone` does.

use std::any::Any;
use std::sync::Arc;

use crate::{Block, BlockPos, ShardCtx};

/// A block entity in the backend's own form: plugins only carry it to a place.
#[derive(Clone)]
pub struct BlockEntityData(Arc<dyn Any + Send + Sync>);

impl BlockEntityData {
    pub fn new<T: Any + Send + Sync>(data: T) -> Self {
        Self(Arc::new(data))
    }

    pub fn get<T: Any>(&self) -> Option<&T> {
        self.0.downcast_ref()
    }
}

impl std::fmt::Debug for BlockEntityData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BlockEntityData")
    }
}

impl ShardCtx<'_> {
    /// What the block at `pos` holds; `None` for a block that holds nothing and a place not loaded.
    pub fn block_entity(&self, pos: BlockPos) -> Option<BlockEntityData> {
        self.backend.block_entity(pos)
    }

    /// Gives the block at `pos` what another held (from [`ShardCtx::block_entity`]), on the screens
    /// of those who see it too. Put the block there first. False where nothing was changed.
    pub fn set_block_entity(&mut self, pos: BlockPos, data: &BlockEntityData) -> bool {
        self.backend.set_block_entity(pos, data)
    }

    /// The other half of the double chest at `pos`, if it is one.
    pub fn paired_with(&self, pos: BlockPos) -> Option<BlockPos> {
        self.backend.paired_with(pos)
    }

    /// Copies blocks as `/clone` does, `cleared` emptied first (a move); shown as the backend's
    /// clone shows them, or else as one [`ShardCtx::batch_blocks`]. The blocks copied, as `/clone` counts
    /// them: a waterlogged block's water too.
    pub fn copy_blocks(&mut self, cleared: &[BlockPos], copies: &[Copied]) -> u32 {
        if let Some(copied) = self.backend.copy_blocks(cleared, copies) {
            return copied;
        }
        self.batch_blocks(|ctx| {
            for pos in cleared {
                ctx.update_block(*pos, &Block::air());
            }
            for copied in copies {
                ctx.update_block(copied.to, &copied.block);
                if let Some(entity) = &copied.entity {
                    ctx.set_block_entity(copied.to, entity);
                }
            }
        });
        copies.len() as u32
    }
}

/// A block [`ShardCtx::copy_blocks`] copies: from where, to where, and what it is and holds there.
#[derive(Clone, Debug)]
pub struct Copied {
    pub from: BlockPos,
    pub to: BlockPos,
    pub block: Block,
    pub entity: Option<BlockEntityData>,
}

/// The block entities side of [`crate::Backend`].
pub trait BlockEntityBackend {
    fn block_entity(&self, pos: BlockPos) -> Option<BlockEntityData>;
    fn set_block_entity(&mut self, pos: BlockPos, data: &BlockEntityData) -> bool;
    /// The other half of the double chest at `pos`, if it is one.
    fn paired_with(&self, _pos: BlockPos) -> Option<BlockPos> {
        None
    }
    /// See [`ShardCtx::copy_blocks`]; `None` leaves it to the batch.
    fn copy_blocks(&mut self, _cleared: &[BlockPos], _copies: &[Copied]) -> Option<u32> {
        None
    }
}

#[cfg(test)]
impl BlockEntityBackend for crate::testing::Fake {
    fn block_entity(&self, _: BlockPos) -> Option<BlockEntityData> {
        None
    }
    fn set_block_entity(&mut self, _: BlockPos, _: &BlockEntityData) -> bool {
        false
    }
}
