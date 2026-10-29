//! Sniffer eggs (`SnifferEggBlock`, docs/research/mob-life.md "Sniffer eggs"): each tick comes 7200
//! to 8799 ticks after the last, half that on moss; it cracks the egg once more with a sparkle, and
//! the tick after the second crack hatches a baby sniffer at the cell's corner.

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::{again, crack, cracks, egg_sound, name_at, roll, sparkle};
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
    if let Some(sparkle) = level(world, layer).filter(|_| on_moss(world, layer, pos)).map(|level| sparkle(level, pos)) {
        send::to_viewers(world, layer, pos, &[sparkle]);
    }
    wait(world, layer, pos);
}

/// The queued tick of the sniffer egg at `pos` (`SnifferEggBlock`'s tick, 146607e10).
pub(super) fn ticked(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(level) = level(world, layer) else { return };
    if cracks(level, pos) < 2 {
        crack(world, layer, pos, "block.sniffer_egg.crack", true);
        return wait(world, layer, pos);
    }
    let air = level.registry().air_id();
    if !send::set(world, layer, pos, air) {
        return;
    }
    // TODO: BDS's one hatch seen came out at z - 0.3025 of the corner (research/captures/bds-sniffers.txt).
    crate::creatures::hatch(world, layer, SNIFFER, pos.map(|c| c as f32));
    send::to_viewers(world, layer, pos, &[egg_sound("block.sniffer_egg.hatch", pos)]);
}
