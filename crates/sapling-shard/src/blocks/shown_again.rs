//! Blocks shown again a tick after they were put, as BDS showed a command block put by
//! `/setblock` or by hand, and the blocks set_entities.rs names: their block and their entity
//! once more, or a bed's entity alone (research/captures/bds-cmdblocks.txt, bds-s7ents.txt).

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::send;
use crate::components::{Layer, Tick};

#[derive(Resource, Default)]
pub(crate) struct ShownAgain(Vec<(u64, Entity, BlockPos, bool)>);

fn queue(world: &mut World, layer: Entity, pos: BlockPos, whole: bool) {
    let due = world.resource::<Tick>().0 + 1;
    world.get_resource_or_insert_with(ShownAgain::default).0.push((due, layer, pos, whole));
}

/// Shows the block at `pos` and its entity again on the next tick.
pub(super) fn next_tick(world: &mut World, layer: Entity, pos: BlockPos) {
    queue(world, layer, pos, true);
}

/// Shows the entity of the block at `pos` again on the next tick, without its block.
pub(super) fn entity_next_tick(world: &mut World, layer: Entity, pos: BlockPos) {
    queue(world, layer, pos, false);
}

/// Shows again what is due, a structure block first put in its entity's mode.
pub(crate) fn show_due(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let Some(mut again) = world.get_resource_mut::<ShownAgain>().filter(|again| !again.0.is_empty()) else { return };
    let (due, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut again.0).into_iter().partition(|(when, ..)| *when <= now);
    again.0 = waiting;
    for (_, layer, pos, whole) in due {
        super::structure_blocks::settle(world, layer, pos);
        super::set_entities::settle(world, layer, pos);
        if whole {
            send::show_whole(world, layer, pos);
            continue;
        }
        let shown = world.get::<Layer>(layer).and_then(|level| level.world.block_entity(pos)).map(|entity| send::block_entity_data(pos, entity));
        send::to_viewers(world, layer, pos, &shown.into_iter().collect::<Vec<_>>());
    }
}
