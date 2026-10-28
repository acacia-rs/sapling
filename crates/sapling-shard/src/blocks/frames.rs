//! What an item frame shows and gives beyond holding its item (holders.rs): a framed map sets the
//! block's `item_frame_map_bit`, and a comparator reads a framed item by its turn, the Java game's
//! `ItemFrame.getAnalogOutput` (docs/research/blocks.md).

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use super::{level, send, with_property};

const FRAMES: [&str; 2] = ["minecraft:frame", "minecraft:glow_frame"];
const MAP_BIT: &str = "item_frame_map_bit";
const MAPS: [&str; 1] = ["minecraft:filled_map"];
const TURN: &str = "ItemRotation";
const EIGHTH: f32 = 45.0;

/// Sets the map bit of the frame at `pos` by what it now holds, `framed` (its entity's `Item`).
pub(super) fn show_map(world: &mut World, layer: Entity, pos: BlockPos, framed: Option<&Value>) {
    let is_map = matches!(framed.and_then(|item| item.get("Name")), Some(Value::String(name)) if MAPS.contains(&name.as_str()));
    let Some(level) = level(world, layer) else { return };
    let id = level.block(pos[0], pos[1], pos[2]);
    if let Some(shown) = with_property(level.registry(), id, MAP_BIT, if is_map { "1" } else { "0" }).filter(|shown| *shown != id) {
        send::set(world, layer, pos, shown);
    }
    super::container_changed(world, layer, pos);
}

/// What a comparator reads off the frame at `pos` that faces `facing`, away from the comparator
/// past the block it hangs on: 1 to 8 by its item's turn, 0 empty; `None` for no such frame.
pub(super) fn signal(world: &World, layer: Entity, pos: BlockPos, facing: Face) -> Option<u8> {
    let level = level(world, layer).filter(|level| level.in_height(pos[1]))?;
    let state = level.registry().get(level.block(pos[0], pos[1], pos[2])).filter(|state| FRAMES.contains(&state.name))?;
    (state.property("facing_direction")? == (facing as usize).to_string()).then_some(())?;
    let kept = level.block_entity(pos).map(|entity| &entity.nbt);
    let Some(kept) = kept.filter(|kept| kept.get("Item").is_some_and(|item| !matches!(item.get("Name"), Some(Value::String(name)) if name.is_empty()))) else { return Some(0) };
    let turn = if let Some(Value::Float(turn)) = kept.get(TURN) { *turn } else { 0.0 };
    Some((turn / EIGHTH).round() as u8 % 8 + 1)
}
