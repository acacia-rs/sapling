//! A brewing stand shows a bottle in each of its three places that holds something
//! (`brewing_stand_slot_a_bit` to `_c_bit` for slots 1 to 3; docs/research/block-entities.md,
//! section 4). It brews nothing.

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::block_name;
use crate::blocks::{set, with_property};
use crate::components::Layer;
use crate::inventory::Held;

const STAND: &str = "minecraft:brewing_stand";
const BOTTLES: [&str; 3] = ["brewing_stand_slot_a_bit", "brewing_stand_slot_b_bit", "brewing_stand_slot_c_bit"];

/// Shows the bottles of the brewing stand at `pos` as `slots` hold them; other blocks are left
/// alone.
pub(super) fn show_bottles(world: &mut World, layer: Entity, pos: BlockPos, slots: &[Option<Held>]) {
    if block_name(world, layer, pos) != Some(STAND) {
        return;
    }
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let before = level.block(pos[0], pos[1], pos[2]);
    let holds = |place: usize| if slots.get(place + 1).is_some_and(Option::is_some) { "1" } else { "0" };
    let now = BOTTLES.iter().enumerate().fold(before, |id, (place, bit)| with_property(level.registry(), id, bit, holds(place)).unwrap_or(id));
    if now != before {
        set(world, layer, pos, now);
    }
}
