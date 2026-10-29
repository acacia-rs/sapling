//! Copper bulbs: each rise of the signal turns one on or off; `powered_bit` remembers the signal
//! (docs/research/redstone-blocks.md, "Copper bulb").

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::{level, send, with_property};

/// The bulb at `pos`, `was` powered, now is or is not.
pub(super) fn powered(world: &mut World, layer: Entity, pos: BlockPos, was: bool, now: bool) {
    if was == now {
        return;
    }
    let Some(level) = level(world, layer) else { return };
    let (registry, id) = (level.registry(), level.block(pos[0], pos[1], pos[2]));
    let lit = registry.get(id).is_some_and(|state| state.properties.split(',').any(|pair| pair == "lit=1"));
    let Some(mut next) = with_property(registry, id, "powered_bit", if now { "1" } else { "0" }) else { return };
    if now {
        next = with_property(registry, next, "lit", if lit { "0" } else { "1" }).unwrap_or(next);
    }
    // At the cell's corner, after the block (research/captures/bds-rsdevices.txt).
    let corner = acacia_proto::types::Vec3f { x: pos[0] as f32, y: pos[1] as f32, z: pos[2] as f32 };
    let sound = now.then(|| send::sound_at(if lit { "block.copper_bulb.turn_off" } else { "block.copper_bulb.turn_on" }, registry, corner, next));
    if send::set(world, layer, pos, next) {
        send::to_viewers(world, layer, pos, &sound.into_iter().collect::<Vec<_>>());
    }
}
