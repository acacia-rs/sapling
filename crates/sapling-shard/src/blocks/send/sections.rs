//! Blocks shown a sub-chunk at a time, in UpdateSubchunkBlocks, as BDS showed a grown tree, the
//! exit portal and a door's two halves.

use acacia_proto::packets::UpdateSubchunkBlocks;
use acacia_proto::types::{BlockCoordinates, BlockUpdate, BlockUpdateTransitionType, UpdateBlockFlags};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::BlockPos;

use super::{FLAGS, to_viewers, update, write};
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
        let block = |&([x, y, z], id, flags): &Cell| BlockUpdate { position: BlockCoordinates { x, y, z }, runtime_id: registry.network_hash(id), flags, entity_unique_id: i64::MIN, transition_type: BlockUpdateTransitionType::Entity };
        sections
            .into_iter()
            .map(|(corner, cells)| match cells.as_slice() {
                [(pos, id, _)] if single_as_block => (*pos, update(registry, *pos, *id, 0)),
                _ => (corner, sapling_net::encode(&UpdateSubchunkBlocks { x: corner[0], y: corner[1], z: corner[2], blocks: cells.iter().map(block).collect(), extra: Vec::new() })),
            })
            .collect()
    };
    for (pos, packet) in packets {
        to_viewers(world, layer, pos, &[packet]);
    }
}
