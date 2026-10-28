//! Blocks shown whole again a tick after they were put, as BDS showed a command block put by
//! `/setblock` or by hand: its block and its entity once more (research/captures/bds-cmdblocks.txt).

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::send;
use crate::components::Tick;

#[derive(Resource, Default)]
pub(crate) struct ShownAgain(Vec<(u64, Entity, BlockPos)>);

/// Shows the block at `pos` and its entity again on the next tick.
pub(super) fn next_tick(world: &mut World, layer: Entity, pos: BlockPos) {
    let due = world.resource::<Tick>().0 + 1;
    world.get_resource_or_insert_with(ShownAgain::default).0.push((due, layer, pos));
}

/// Shows again what is due, a structure block first put in its entity's mode.
pub(crate) fn show_due(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let Some(mut again) = world.get_resource_mut::<ShownAgain>().filter(|again| !again.0.is_empty()) else { return };
    let (due, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut again.0).into_iter().partition(|(when, ..)| *when <= now);
    again.0 = waiting;
    for (_, layer, pos) in due {
        super::structure_blocks::settle(world, layer, pos);
        send::show_whole(world, layer, pos);
    }
}
