//! Sniffer eggs (`SnifferEggBlock`, docs/research/mob-life.md "Sniffer eggs"): each tick comes 7200
//! to 8799 ticks after the last, half that on moss; it cracks the egg once more with a sparkle, and
//! the tick after the second crack hatches a baby sniffer.

use acacia_proto::packets::LevelEventEvent;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::{again, crack, cracks, level_event, name_at, roll};
use crate::blocks::{level, send};

pub(super) const EGG: &str = "minecraft:sniffer_egg";
const SNIFFER: &str = "minecraft:sniffer";
const MOSS: &str = "minecraft:moss_block";
/// Ticks to the next tick: the least, and how many more it may be; on moss it is halved.
const NEXT_TICK: (u64, u32) = (7200, 1600);

fn on_moss(world: &World, layer: Entity, [x, y, z]: BlockPos) -> bool {
    level(world, layer).and_then(|level| name_at(level, [x, y - 1, z])) == Some(MOSS)
}

/// Queues the egg at `pos`'s next tick.
fn wait(world: &mut World, layer: Entity, pos: BlockPos) {
    let delay = NEXT_TICK.0 + u64::from(roll(world, NEXT_TICK.1));
    again(world, layer, pos, if on_moss(world, layer, pos) { delay / 2 } else { delay });
}

/// A sniffer egg came at `pos`: on moss it sparkles (146607830).
pub(super) fn placed(world: &mut World, layer: Entity, pos: BlockPos) {
    if on_moss(world, layer, pos) {
        send::to_viewers(world, layer, pos, &[level_event(LevelEventEvent::ParticleTurtleEgg, pos, 0)]);
    }
    wait(world, layer, pos);
}

/// The queued tick of the sniffer egg at `pos` (`SnifferEggBlock`'s tick, 146607e10).
pub(super) fn ticked(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(level) = level(world, layer) else { return };
    if cracks(level, pos) < 2 {
        send::to_viewers(world, layer, pos, &[level_event(LevelEventEvent::ParticleTurtleEgg, pos, 0)]);
        crack(world, layer, pos, "block.sniffer_egg.crack");
        return wait(world, layer, pos);
    }
    let [x, y, z] = pos;
    let (id, air) = (level.block(x, y, z), level.registry().air_id());
    let heard = send::sound("block.sniffer_egg.hatch", level.registry(), pos, id);
    if !send::set(world, layer, pos, air) {
        return;
    }
    send::to_viewers(world, layer, pos, &[heard]);
    crate::creatures::hatch(world, layer, SNIFFER, [x as f32 + 0.5, y as f32, z as f32 + 0.5]);
}
