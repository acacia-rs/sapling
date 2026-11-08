//! Blocks shown again a tick after they were put, as BDS showed a command block put by
//! `/setblock` or by hand, and the blocks set_entities.rs names: their block and their entity
//! once more (research/captures/bds-cmdblocks.txt, bds-s7ents.txt). A set bed's halves are shown
//! again together, foot first, then their entities, and the foot is heard placed (bds-e3sets.txt).

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::send;
use crate::components::{Layer, Tick};

#[derive(Resource, Default)]
pub(crate) struct ShownAgain(Vec<(u64, Entity, BlockPos, bool)>);

fn queue(world: &mut World, layer: Entity, pos: BlockPos, whole: bool, ticks: u64) {
    let due = world.resource::<Tick>().0 + ticks;
    world.get_resource_or_insert_with(ShownAgain::default).0.push((due, layer, pos, whole));
}

/// Shows the block at `pos` and its entity again on the next tick.
pub(super) fn next_tick(world: &mut World, layer: Entity, pos: BlockPos) {
    queue(world, layer, pos, true, 1);
}

/// Shows the block at `pos` and its entity again on each of the next `ticks` ticks.
pub(super) fn each_tick(world: &mut World, layer: Entity, pos: BlockPos, ticks: u64) {
    (1..=ticks).for_each(|after| queue(world, layer, pos, true, after));
}

/// Shows the bed half at `pos` again on the next tick, with the other halves due then.
pub(super) fn bed_next_tick(world: &mut World, layer: Entity, pos: BlockPos) {
    queue(world, layer, pos, false, 1);
}

/// Double chests paired a tick ago: when, the world, the lead half and the other.
#[derive(Resource, Default)]
pub(crate) struct PairsShown(Vec<(u64, Entity, [BlockPos; 2])>);

/// Shows the chest pair `halves` (lead first) again on the next tick, as BDS did once a chest put
/// beside another paired with it: lead, other, other, lead in one UpdateSubchunkBlocks, then their
/// entities in that order (research/captures/bds-u3chest.txt, bds-r3copper.txt).
pub(crate) fn pair_next_tick(world: &mut World, layer: Entity, halves: [BlockPos; 2]) {
    let due = world.resource::<Tick>().0 + 1;
    world.get_resource_or_insert_with(PairsShown::default).0.push((due, layer, halves));
}

fn show_pairs(world: &mut World, now: u64) {
    let Some(mut pairs) = world.get_resource_mut::<PairsShown>().filter(|pairs| !pairs.0.is_empty()) else { return };
    let (due, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut pairs.0).into_iter().partition(|(when, ..)| *when <= now);
    pairs.0 = waiting;
    for (_, layer, [lead, other]) in due {
        let cells = [lead, other, other, lead];
        send::show_together(world, layer, &cells.map(|pos| (pos, send::FLAGS)));
        let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { continue };
        let entities: Vec<_> = cells.iter().filter_map(|pos| level.block_entity(*pos).map(|entity| send::block_entity_data(*pos, entity))).collect();
        send::to_viewers(world, layer, lead, &entities);
    }
}

/// Shows again what is due, a structure block first put in its entity's mode.
pub(crate) fn show_due(world: &mut World) {
    let now = world.resource::<Tick>().0;
    show_pairs(world, now);
    let Some(mut again) = world.get_resource_mut::<ShownAgain>().filter(|again| !again.0.is_empty()) else { return };
    let (due, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut again.0).into_iter().partition(|(when, ..)| *when <= now);
    again.0 = waiting;
    let mut beds: Vec<(Entity, BlockPos)> = Vec::new();
    for (_, layer, pos, whole) in due {
        super::structure_blocks::settle(world, layer, pos);
        super::set_entities::settle(world, layer, pos);
        if whole && super::set_entities::is_unheld_frame(world, layer, pos) {
            super::updates::pop_unshown(world, layer, pos);
        } else if whole {
            send::show_whole(world, layer, pos);
        } else {
            beds.insert(0, (layer, pos));
        }
    }
    show_beds(world, &beds);
}

fn show_beds(world: &mut World, beds: &[(Entity, BlockPos)]) {
    let Some(&(layer, _)) = beds.first() else { return };
    let cells: Vec<BlockPos> = beds.iter().filter(|(at, _)| *at == layer).map(|(_, pos)| *pos).collect();
    send::show_together(world, layer, &cells.iter().map(|pos| (*pos, send::FLAGS)).collect::<Vec<_>>());
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let entities: Vec<_> = cells.iter().filter_map(|pos| level.block_entity(*pos).map(|entity| (*pos, send::block_entity_data(*pos, entity)))).collect();
    let is_foot = |[x, y, z]: BlockPos| level.registry().get(level.block(x, y, z)).is_some_and(|state| state.properties.contains("head_piece_bit=0"));
    let feet: Vec<BlockPos> = cells.iter().copied().filter(|pos| is_foot(*pos)).collect();
    for (pos, entity) in entities {
        send::to_viewers(world, layer, pos, &[entity]);
    }
    for foot in feet {
        super::vibrate_at(world, layer, foot, super::GameEvent::BlockPlace, false);
    }
    show_beds(world, &beds.iter().copied().filter(|(at, _)| *at != layer).collect::<Vec<_>>());
}
