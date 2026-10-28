//! Bells, as BDS rang them (research/captures/bds-bells.txt, docs/research/block-entities.md
//! "Bells"): a click on any face but a standing bell's top or bottom, or the rise of its signal,
//! shows the entity `Ringing` 1 with the `Direction` it swings to and plays `block.bell.hit` at
//! the block's corner; a click also flips `toggle_bit`.
// TODO: BDS clears `Ringing` when the swing ends, unshown; here it stays 1 until the next ring.
// Raiders are not lit: Bedrock has no glowing to light them with [U: what BDS's bell does for raids].

use acacia_proto::nbt::Value;
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};
use sapling_world::new_block_entity;

use super::send::keep_entity;
use super::{level, send, with_property};
use crate::components::Position;

const BELL: &str = "minecraft:bell";
const ENTITY: &str = "Bell";
/// What a bell's entity holds before it ever rang.
pub(super) const UNRUNG: [(&str, Value); 3] = [("Direction", Value::Int(255)), ("Ringing", Value::Byte(0)), ("Ticks", Value::Int(0))];

/// The horizontal way, as `direction` numbers it (south, west, north, east), that points from
/// `from` most nearly toward `to`.
fn way(from: [f32; 2], to: [f32; 2]) -> i32 {
    let [dx, dz] = [to[0] - from[0], to[1] - from[1]];
    match dx.abs() > dz.abs() {
        true if dx > 0.0 => 3,
        true => 1,
        false if dz > 0.0 => 0,
        false => 2,
    }
}

/// The bell's state and `attachment` at `pos`, if a bell is there.
fn bell_at(world: &World, layer: Entity, pos: BlockPos) -> Option<(u32, String)> {
    let level = level(world, layer)?;
    let id = level.block(pos[0], pos[1], pos[2]);
    let state = level.registry().get(id).filter(|state| state.name == BELL)?;
    Some((id, state.property("attachment")?.to_owned()))
}

fn ring(world: &mut World, layer: Entity, pos: BlockPos, direction: i32) {
    let mut fields = match level(world, layer).and_then(|level| level.block_entity(pos)) {
        Some(entity) => entity.nbt.clone(),
        None => new_block_entity(ENTITY, pos),
    };
    if let Value::Compound(kept) = &mut fields {
        kept.retain(|(name, _)| !matches!(name.as_str(), "Direction" | "Ringing" | "Ticks"));
        kept.extend([("Direction".into(), Value::Int(direction)), ("Ringing".into(), Value::Byte(1)), ("Ticks".into(), Value::Int(0))]);
    }
    keep_entity(world, layer, pos, &fields);
    crate::creatures::hear_bell(world, layer, pos);
    let [x, y, z] = pos.map(|c| c as f32);
    send::to_viewers(world, layer, pos, &[send::noise("block.bell.hit", Vec3f { x, y, z }, -1)]);
}

/// A player's click on `face` of the block `name` at `pos`: false when it is no bell or the
/// face does not ring it. `Direction` is the way toward the player.
pub(super) fn ring_by_hand(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, face: Face, name: &str) -> bool {
    if name != BELL {
        return false;
    }
    let Some((id, attachment)) = bell_at(world, layer, pos) else { return false };
    if attachment == "standing" && matches!(face, Face::Up | Face::Down) {
        return false;
    }
    let Some(feet) = world.get::<Position>(player).map(|position| position.feet) else { return false };
    let flipped = level(world, layer).and_then(|level| {
        let toggled = level.registry().get(id)?.property("toggle_bit")? == "0";
        with_property(level.registry(), id, "toggle_bit", if toggled { "1" } else { "0" })
    });
    if let Some(flipped) = flipped {
        send::set(world, layer, pos, flipped);
    }
    ring(world, layer, pos, way([pos[0] as f32 + 0.5, pos[2] as f32 + 0.5], [feet[0], feet[2]]));
    true
}

/// A bell's signal rose: it rings toward the way it faces.
// The drill's one powered bell faced south with the power south of it; which of the two
// `Direction` follows is not drilled.
pub(crate) fn ring_by_power(world: &mut World, layer: Entity, pos: BlockPos) {
    let facing = level(world, layer).and_then(|level| level.registry().get(level.block(pos[0], pos[1], pos[2]))?.property("direction")?.parse().ok());
    ring(world, layer, pos, facing.unwrap_or(0));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bell_swings_away_from_the_player_as_on_bds() {
        // bds-bells.txt: the player at (0.5, 0.5); bells north-east, south-east and east of it.
        let player = [0.5, 0.5];
        assert_eq!([way([2.5, -2.5], player), way([2.5, 3.5], player), way([6.5, 0.5], player)], [0, 2, 1]);
    }
}
