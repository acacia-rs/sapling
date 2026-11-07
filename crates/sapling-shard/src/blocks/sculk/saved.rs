//! A sensor's or shrieker's block actor, as BDS saves it (`SculkSensorBlockActor` 14487a960,
//! `SculkShriekerBlockActor` 14487d530; a BDS world dumped after `bds_command_probe uusculk`):
//! `BlockEntityVersion` 0 and a `VibrationListener` holding an empty `selector` and, once it heard
//! one, the type of the last `event` it heard (sculk.md's table).
// TODO: a vibration on its way (`ticks`, `pending`) and a shrieker's `QueuedResponseThreatLevel` are not kept.

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::new_block_entity;

use super::{CALIBRATED, GameEvent, SENSOR, SHRIEKER};
use crate::components::Layer;

const LISTENER: &str = "VibrationListener";
const EVENT: &str = "event";

fn entity_id(name: &str) -> Option<&'static str> {
    match name {
        SENSOR => Some("SculkSensor"),
        CALIBRATED => Some("CalibratedSculkSensor"),
        SHRIEKER => Some("SculkShrieker"),
        _ => None,
    }
}

fn fresh(id: &str, pos: BlockPos) -> Value {
    let mut entity = new_block_entity(id, pos);
    if let Value::Compound(fields) = &mut entity {
        fields.push(("BlockEntityVersion".into(), Value::Int(0)));
        fields.push((LISTENER.into(), Value::Compound(vec![("selector".into(), Value::Compound(Vec::new()))])));
    }
    entity
}

fn store(world: &mut World, layer: Entity, pos: BlockPos, entity: &Value) {
    if let Some(mut level) = world.get_mut::<Layer>(layer) {
        level.world.set_block_entity(pos, entity);
    }
}

/// The listener `name` was written at `pos`: it holds a fresh block actor.
pub(super) fn placed(world: &mut World, layer: Entity, pos: BlockPos, name: &str) {
    if let Some(id) = entity_id(name) {
        store(world, layer, pos, &fresh(id, pos));
    }
}

/// The listener `name` at `pos` heard `event`: its block actor keeps it.
pub(super) fn heard(world: &mut World, layer: Entity, pos: BlockPos, name: &str, event: GameEvent) {
    let Some(id) = entity_id(name) else { return };
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let kept = level.block_entity(pos).map(|entity| entity.nbt.clone()).filter(|nbt| matches!(nbt.get("id"), Some(Value::String(kept)) if kept.as_str() == id));
    let mut entity = kept.unwrap_or_else(|| fresh(id, pos));
    let Value::Compound(fields) = &mut entity else { return };
    let listener = match fields.iter_mut().find(|(key, _)| key == LISTENER) {
        Some((_, listener)) => listener,
        None => {
            fields.push((LISTENER.into(), Value::Compound(Vec::new())));
            &mut fields.last_mut().expect("just pushed").1
        }
    };
    let Value::Compound(listened) = listener else { return };
    listened.retain(|(key, _)| key != EVENT);
    listened.push((EVENT.into(), Value::Int(event.type_id())));
    store(world, layer, pos, &entity);
}
