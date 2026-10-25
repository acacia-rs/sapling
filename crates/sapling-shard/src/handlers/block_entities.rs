//! The ctx calls that carry a block entity from one place to another (`/clone`).

use acacia_proto::nbt::Value;
use sapling_api::{BlockEntityBackend, BlockEntityData, BlockPos};

use super::Ctx;
use crate::components::Layer;
use crate::{blocks, worlds};

impl BlockEntityBackend for Ctx<'_> {
    fn block_entity(&self, pos: BlockPos) -> Option<BlockEntityData> {
        self.layer().world.block_entity(pos).map(|entity| BlockEntityData::new(entity.nbt.clone()))
    }

    fn set_block_entity(&mut self, pos: BlockPos, data: &BlockEntityData) -> bool {
        let Some(entity) = data.get::<Value>() else { return false };
        let layer = worlds::current(self.world);
        if self.world.get::<Layer>(layer).is_none_or(|layer| layer.world.block(pos[0], pos[1], pos[2]) == layer.world.registry().air_id()) {
            return false;
        }
        blocks::keep_entity(self.world, layer, pos, &blocks::block_entity_at(entity.clone(), pos));
        true
    }
}
