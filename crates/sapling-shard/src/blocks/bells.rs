//! Bells, as BDS rang them (research/captures/bds-bells.txt, docs/research/block-entities.md
//! "Bells"): a click on any face but a standing bell's top or bottom, or the rise of its signal,
//! shows the entity `Ringing` 1 with the `Direction` it swings to and plays `block.bell.hit` at
//! the block's corner; a click also flips `toggle_bit`. While its column ticks, a bell counts
//! `Ticks` up from its placing or last ring and wraps at 50, unshown (bds-t3bells.txt); the
//! wrap ends a swing's `Ringing`, BDS's clear itself never shown.
// Raiders are not lit: Bedrock has no glowing to light them with [U: what BDS's bell does for raids].

use std::collections::HashSet;

use acacia_proto::nbt::Value;
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};
use sapling_world::new_block_entity;

use super::send::keep_entity;
use super::{level, send, with_property};
use crate::components::{Layer, Position};

const BELL: &str = "minecraft:bell";
const ENTITY: &str = "Bell";
/// What a bell's entity holds before it ever rang.
pub(super) const UNRUNG: [(&str, Value); 3] = [("Direction", Value::Int(255)), ("Ringing", Value::Byte(0)), ("Ticks", Value::Int(0))];
const SWING_TICKS: i32 = 50;

/// The bells that count their `Ticks`.
#[derive(Resource, Default)]
struct Counting(HashSet<(Entity, BlockPos)>);

pub(crate) fn watch(world: &mut World, layer: Entity, pos: BlockPos) {
    world.get_resource_or_insert_with(Counting::default).0.insert((layer, pos));
}

pub(crate) fn watched(world: &World) -> Vec<(Entity, BlockPos)> {
    world.get_resource::<Counting>().map(|counting| counting.0.iter().copied().collect()).unwrap_or_default()
}

pub(crate) fn forget(world: &mut World, layer: Entity, pos: BlockPos) {
    if let Some(mut counting) = world.get_resource_mut::<Counting>() {
        counting.0.remove(&(layer, pos));
    }
}

/// The bell `id` put at `pos` on the `face` clicked: on a side it faces off that face whatever the
/// placer's look, and with a solid block across the cell it hangs between the two
/// (`attachment=multiple`; bds-p3bell.txt).
pub(super) fn placed(level: &sapling_world::World, pos: BlockPos, id: u32, face: Face) -> u32 {
    let registry = level.registry();
    let direction = match face {
        Face::South => "0",
        Face::West => "1",
        Face::North => "2",
        Face::East => "3",
        Face::Up | Face::Down => return id,
    };
    if !registry.get(id).is_some_and(|state| state.name == BELL) {
        return id;
    }
    let across = face.neighbour(pos);
    let between = level.in_height(across[1]) && registry.get(level.block(across[0], across[1], across[2])).is_some_and(|state| state.is_solid());
    let id = with_property(registry, id, "direction", direction).unwrap_or(id);
    if between { with_property(registry, id, "attachment", "multiple").unwrap_or(id) } else { id }
}

/// Starts counting a bell just put at `pos`.
pub(super) fn noticed(world: &mut World, layer: Entity, pos: BlockPos) {
    if bell_at(world, layer, pos).is_some() {
        watch(world, layer, pos);
    }
}

/// One tick of every counting bell's `Ticks`, without showing it; a cell no longer a bell stops.
pub(crate) fn count(world: &mut World) {
    for (layer, pos) in watched(world) {
        if bell_at(world, layer, pos).is_none() {
            forget(world, layer, pos);
            continue;
        }
        let Some(mut fields) = level(world, layer).and_then(|level| level.block_entity(pos)).map(|entity| entity.nbt.clone()) else { continue };
        let ticks = match fields.get("Ticks") {
            Some(Value::Int(ticks)) => ticks + 1,
            _ => 1,
        };
        let wrapped = ticks >= SWING_TICKS;
        if let Value::Compound(kept) = &mut fields {
            for (name, value) in kept.iter_mut() {
                match name.as_str() {
                    "Ticks" => *value = Value::Int(if wrapped { 0 } else { ticks }),
                    "Ringing" if wrapped => *value = Value::Byte(0),
                    _ => {}
                }
            }
        }
        if let Some(mut level) = world.get_mut::<Layer>(layer) {
            level.world.set_block_entity(pos, &fields);
        }
    }
}

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
    watch(world, layer, pos);
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

    /// bds-p3bell.txt: put on a stone's north face, alone and with another stone across the gap.
    #[test]
    fn a_bell_between_two_blocks_hangs_from_both() {
        let mut level = sapling_world::World::void();
        let (stone, bell) = (level.block_id("minecraft:stone").unwrap(), level.block_id("minecraft:bell").unwrap());
        level.set_block(3, 64, 1, stone);
        let state = |level: &sapling_world::World| level.registry().get(placed(level, [3, 64, 0], bell, Face::North)).unwrap().properties;
        assert!(!state(&level).contains("attachment=multiple") && state(&level).contains("direction=2"), "{}", state(&level));
        level.set_block(3, 64, -1, stone);
        assert!(state(&level).contains("attachment=multiple"), "{}", state(&level));
    }
}
