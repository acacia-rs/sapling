//! What a block holds beyond its state (a chest's items, a sign's text), carried whole from one
//! place to another, as `/clone` does.

use std::any::Any;
use std::sync::Arc;

use crate::{BlockPos, ShardCtx};

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
}

/// The block entities side of [`crate::Backend`].
pub trait BlockEntityBackend {
    fn block_entity(&self, pos: BlockPos) -> Option<BlockEntityData>;
    fn set_block_entity(&mut self, pos: BlockPos, data: &BlockEntityData) -> bool;
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
