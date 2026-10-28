//! Blocks shown a sub-chunk at a time, in UpdateSubchunkBlocks, as BDS showed a grown tree, the
//! exit portal, a door's two halves and a `/fill` (a plugin's batch, held until it is done).

use acacia_proto::packets::UpdateSubchunkBlocks;
use acacia_proto::types::{BlockCoordinates, BlockUpdate, BlockUpdateTransitionType, UpdateBlockFlags};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::BlockPos;

use super::{FLAGS, block_entity_data, to_viewers, update_flagged, write};
use crate::components::Layer;

/// A cell, the block shown in it, and the flags it is shown with.
type Cell = (BlockPos, u32, u32);

/// Writes `cells` as [`super::set_quietly`] does, in order, and shows the ones that changed as BDS
/// showed a grown tree: an UpdateSubchunkBlocks per sub-chunk, sub-chunks as first written
/// (research/captures/bds-grows.txt). How many changed.
pub(crate) fn set_many(world: &mut World, layer: Entity, cells: &[(BlockPos, u32)]) -> usize {
    let sections = write_by_section(world, layer, cells);
    let changed = sections.iter().map(|(_, cells)| cells.len()).sum();
    show_sections(world, layer, sections, false);
    changed
}

/// [`set_many`], but a sub-chunk with one change is shown by an UpdateBlock, those first, as BDS
/// showed the exit portal (research/captures/bds-dragon-fight.txt).
pub(crate) fn set_sectioned(world: &mut World, layer: Entity, cells: &[(BlockPos, u32)]) {
    let mut sections = write_by_section(world, layer, cells);
    sections.sort_by_key(|(_, cells)| cells.len() > 1);
    show_sections(world, layer, sections, true);
}

/// Shows the blocks at `cells` as they are, each with its flags, an UpdateSubchunkBlocks per
/// sub-chunk, as BDS showed a door's two halves (research/captures/bds-held-use.txt).
pub(crate) fn show_together(world: &mut World, layer: Entity, cells: &[(BlockPos, UpdateBlockFlags)]) {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let shown = cells.iter().filter(|(pos, _)| level.in_height(pos[1])).map(|&(pos, flags)| (pos, level.block(pos[0], pos[1], pos[2]), flags.0));
    let sections = by_section(shown);
    show_sections(world, layer, sections, false);
}

/// Shows `cells` as given, without writing them, an UpdateSubchunkBlocks per sub-chunk: a cell
/// may be listed twice, as BDS showed a landed stalactite put and broken (bds-dripstone.txt).
pub(crate) fn show_states(world: &mut World, layer: Entity, cells: &[(BlockPos, u32)]) {
    let sections = by_section(cells.iter().map(|&(pos, id)| (pos, id, FLAGS.0)));
    show_sections(world, layer, sections, false);
}

/// The cells [`super::set`] wrote while a batch was open, and the flags they are shown with.
#[derive(Resource, Default)]
pub(crate) struct Held(Option<(UpdateBlockFlags, Vec<(Entity, BlockPos)>)>);

/// Opens a batch whose blocks are shown with `flags`: [`super::set`] keeps what it shows until
/// [`show_held`]. False where one is open already, which then takes these changes in too.
pub(crate) fn hold(world: &mut World, flags: UpdateBlockFlags) -> bool {
    let mut held = world.get_resource_or_insert_with(Held::default);
    held.0.is_none() && held.0.replace((flags, Vec::new())).is_none()
}

/// Runs `system` in a batch of its own, its blocks shown with both flags, as BDS shows what flows
/// and what a circuit changes in a tick (research/captures/bds-concrete.txt, bds-redstone.txt).
pub(crate) fn batched(world: &mut World, system: impl FnOnce(&mut World)) {
    let opened = hold(world, FLAGS);
    system(world);
    if opened {
        show_held(world);
    }
}

/// Whether a batch is open, which then keeps the cell [`super::set`] wrote to show later.
pub(super) fn held(world: &mut World, layer: Entity, pos: BlockPos) -> bool {
    let Some((_, cells)) = world.get_resource_mut::<Held>().and_then(|held| held.into_inner().0.as_mut()) else { return false };
    cells.push((layer, pos));
    true
}

/// Closes the batch and shows what it changed as BDS showed a `/fill` (research/captures/bds-batch.txt):
/// as [`set_sectioned`] does, but each cell as it is now and with the batch's flags (a plugin's: the
/// network flag alone), and then their block entities, which an UpdateBlock would otherwise empty.
pub(crate) fn show_held(world: &mut World) {
    let Some((flags, cells)) = world.get_resource_mut::<Held>().and_then(|mut held| held.0.take()) else { return };
    let mut layers: Vec<Entity> = cells.iter().map(|(layer, _)| *layer).collect();
    layers.dedup();
    for layer in layers {
        let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { continue };
        let mut seen = std::collections::HashSet::new();
        let mut entities: Vec<(BlockPos, Bytes)> = Vec::new();
        let mut shown = Vec::new();
        for &(_, pos) in cells.iter().filter(|(at, _)| *at == layer) {
            if seen.insert(pos) {
                shown.push(pos);
                entities.extend(level.block_entity(pos).map(|entity| (pos, block_entity_data(pos, entity))));
            }
        }
        let mut sections = by_section(shown.iter().map(|&pos| (pos, level.block(pos[0], pos[1], pos[2]), flags.0)));
        sections.sort_by_key(|(_, cells)| cells.len() > 1);
        show_sections(world, layer, sections, true);
        for (pos, packet) in entities {
            to_viewers(world, layer, pos, &[packet]);
        }
    }
}

/// Writes `cells` in order; those that changed, by sub-chunk in the order first written.
fn write_by_section(world: &mut World, layer: Entity, cells: &[(BlockPos, u32)]) -> Vec<(BlockPos, Vec<Cell>)> {
    let written: Vec<Cell> = cells.iter().filter_map(|&(pos, id)| Some((pos, write(world, layer, pos, id)?, FLAGS.0))).collect();
    by_section(written.into_iter())
}

fn by_section(cells: impl Iterator<Item = Cell>) -> Vec<(BlockPos, Vec<Cell>)> {
    let mut sections: Vec<(BlockPos, Vec<Cell>)> = Vec::new();
    for cell in cells {
        let section = cell.0.map(|c| c.div_euclid(16) * 16);
        match sections.iter_mut().find(|(at, _)| *at == section) {
            Some((_, cells)) => cells.push(cell),
            None => sections.push((section, vec![cell])),
        }
    }
    sections
}

fn show_sections(world: &mut World, layer: Entity, sections: Vec<(BlockPos, Vec<Cell>)>, single_as_block: bool) {
    let packets: Vec<(BlockPos, Bytes)> = {
        let Some(registry) = world.get::<Layer>(layer).map(|level| level.world.registry()) else { return };
        let block = |&([x, y, z], id, flags): &Cell| BlockUpdate { position: BlockCoordinates { x, y, z }, runtime_id: sapling_world::block_hash(registry, id), flags, entity_unique_id: i64::MIN, transition_type: BlockUpdateTransitionType::Entity };
        sections
            .into_iter()
            .map(|(corner, cells)| match cells.as_slice() {
                [(pos, id, flags)] if single_as_block => (*pos, update_flagged(registry, *pos, *id, 0, UpdateBlockFlags(*flags))),
                _ => (corner, sapling_net::encode(&UpdateSubchunkBlocks { x: corner[0], y: corner[1], z: corner[2], blocks: cells.iter().map(block).collect(), extra: Vec::new() })),
            })
            .collect()
    };
    for (pos, packet) in packets {
        to_viewers(world, layer, pos, &[packet]);
    }
}
