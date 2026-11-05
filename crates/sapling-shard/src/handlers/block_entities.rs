//! The ctx calls that carry a block entity from one place to another (`/clone`).

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_api::{BlockEntityBackend, BlockEntityData, BlockPos, Copied};

use super::Ctx;
use crate::blocks::cloned;
use crate::components::Layer;
use crate::{blocks, worlds};

impl BlockEntityBackend for Ctx<'_> {
    fn block_entity(&self, pos: BlockPos) -> Option<BlockEntityData> {
        self.layer().world.block_entity(pos).map(|entity| BlockEntityData::new(entity.nbt.clone()))
    }

    fn set_block_entity(&mut self, pos: BlockPos, data: &BlockEntityData) -> bool {
        let Some(entity) = data.get::<Value>() else { return false };
        keep(self.world, worlds::current(self.world), pos, entity)
    }

    fn copy_blocks(&mut self, cleared: &[BlockPos], copies: &[Copied]) -> Option<u32> {
        let layer = worlds::current(self.world);
        let copies: Vec<cloned::Copied<'_>> = copies.iter().map(|copy| cloned::Copied { from: copy.from, to: copy.to, block: &copy.block, entity: copy.entity.as_ref().and_then(|entity| entity.get::<Value>()) }).collect();
        Some(cloned::copy(self.world, layer, cleared, &copies, |world, pos, entity| _ = keep(world, layer, pos, entity)))
    }

    fn paired_with(&self, pos: BlockPos) -> Option<BlockPos> {
        let halves = crate::containers::chest_halves(self.world, worlds::current(self.world), pos);
        halves.into_iter().find(|half| *half != pos)
    }
}

/// Gives the block at `pos` the entity another held, as carried there; false on air.
fn keep(world: &mut World, layer: Entity, pos: BlockPos, entity: &Value) -> bool {
    if world.get::<Layer>(layer).is_none_or(|layer| layer.world.block(pos[0], pos[1], pos[2]) == layer.world.registry().air_id()) {
        return false;
    }
    blocks::keep_entity(world, layer, pos, &blocks::block_entity_at(crate::containers::carried(entity, pos), pos));
    if crate::spawners::is_spawner(entity) {
        crate::spawners::watch(world, layer, pos);
    }
    true
}
