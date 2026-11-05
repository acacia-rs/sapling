//! Copper golem statues: the entity each holds, set or put by hand, and the turn of its pose an
//! empty hand gives it (research/captures/bds-v2statue.txt, bds-z2open.txt).

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::new_block_entity;

use super::send;
use crate::components::Layer;

const POSE: &str = "Pose";
const POSES: i32 = 4;

/// Whether `block` is a copper golem statue, of any stage or wax.
pub(super) fn is_statue(block: &str) -> bool {
    block.ends_with("copper_golem_statue")
}

/// The entity BDS gave every statue, set or put by hand: an unnamed golem in the first pose.
pub(super) fn entity(pos: BlockPos) -> Value {
    let golem = Value::Compound(vec![("ActorIdentifier".into(), Value::String("minecraft:copper_golem<>".into())), ("SaveData".into(), Value::Compound(Vec::new()))]);
    let mut entity = new_block_entity("CopperGolemStatue", pos);
    if let Value::Compound(fields) = &mut entity {
        fields.extend([("Actor".into(), golem), (POSE.into(), Value::Int(0))]);
    }
    entity
}

/// A click with nothing usable in hand: the statue at `pos` takes its next pose and is shown, block
/// and entity, at once and with no sound (BDS 1471397a0, W8 in copper.md).
pub(super) fn turn(world: &mut World, layer: Entity, pos: BlockPos) -> bool {
    let Some(mut level) = world.get_mut::<Layer>(layer) else { return false };
    let mut kept = level.world.block_entity(pos).map_or_else(|| entity(pos), |kept| kept.nbt.clone());
    let pose = match kept.get(POSE) {
        Some(Value::Int(pose)) => *pose,
        _ => 0,
    };
    if let Value::Compound(fields) = &mut kept {
        fields.retain(|(key, _)| key != POSE);
        fields.push((POSE.into(), Value::Int((pose + 1) % POSES)));
    }
    _ = level.world.set_block_entity(pos, &kept);
    send::show_whole(world, layer, pos);
    true
}
