//! Single fields of a block's entity, read and written in place: a note block's note, a jukebox's
//! disc, a crafter's disabled slots.

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::new_block_entity;

use super::{keep_entity, level};

/// The field `key` of the entity of the block at `pos`.
pub(crate) fn get(world: &World, layer: Entity, pos: BlockPos, key: &str) -> Option<Value> {
    level(world, layer)?.block_entity(pos)?.nbt.get(key).cloned()
}

/// The byte `key` of the entity of the block at `pos`.
pub(crate) fn byte(world: &World, layer: Entity, pos: BlockPos, key: &str) -> Option<i8> {
    match get(world, layer, pos, key)? {
        Value::Byte(value) => Some(value),
        _ => None,
    }
}

/// Sets `key` of the entity of the block at `pos` (`None` takes it out), making an entity with the
/// `id` if it has none, and shows it to those who see the block.
pub(crate) fn set(world: &mut World, layer: Entity, pos: BlockPos, id: &str, key: &str, value: Option<Value>) {
    let Some(level) = level(world, layer) else { return };
    let entity = level.block_entity(pos).map_or_else(|| new_block_entity(id, pos), |entity| entity.nbt.clone());
    let Value::Compound(mut fields) = entity else { return };
    fields.retain(|(name, _)| name != key);
    fields.extend(value.map(|value| (key.into(), value)));
    keep_entity(world, layer, pos, &Value::Compound(fields));
}
