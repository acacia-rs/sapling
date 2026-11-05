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
    set_many(world, layer, pos, id, &[(key, value)]);
}

/// [`set`] for several fields at once, shown together.
pub(crate) fn set_many(world: &mut World, layer: Entity, pos: BlockPos, id: &str, values: &[(&str, Option<Value>)]) {
    if let Some(entity) = with_fields(world, layer, pos, id, values) {
        keep_entity(world, layer, pos, &entity);
    }
}

/// [`set`] without showing it.
pub(crate) fn set_quietly(world: &mut World, layer: Entity, pos: BlockPos, id: &str, key: &str, value: Option<Value>) {
    if let Some(entity) = with_fields(world, layer, pos, id, &[(key, value)]) {
        _ = super::send::keep_entity_quietly(world, layer, pos, &entity);
    }
}

fn with_fields(world: &World, layer: Entity, pos: BlockPos, id: &str, values: &[(&str, Option<Value>)]) -> Option<Value> {
    let level = level(world, layer)?;
    let entity = level.block_entity(pos).map_or_else(|| new_block_entity(id, pos), |entity| entity.nbt.clone());
    let Value::Compound(mut fields) = entity else { return None };
    for (key, value) in values {
        fields.retain(|(name, _)| name != key);
        fields.extend(value.clone().map(|value| ((*key).into(), value)));
    }
    Some(Value::Compound(fields))
}
