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

/// The writes kept while [`logged`] runs: each one's world, cell, layer (0 or 1) and block written.
#[derive(Resource, Default)]
pub(crate) struct Logged(Option<Vec<(Entity, BlockPos, u32, u32)>>);

/// Runs `writes` keeping every block [`super::set`] and [`super::set_liquid`] write, then shows them
/// as BDS shows a sub-chunk's writes of a tick (bds-plantpop2.txt, bds-fixset.txt): each write
/// listed in the order made, layer 1's as `extra`, in one UpdateSubchunkBlocks, or an UpdateBlock
/// for a write alone. Inside an open batch or log, that takes the writes instead.
pub(crate) fn logged<R>(world: &mut World, writes: impl FnOnce(&mut World) -> R) -> R {
    let held = world.get_resource::<Held>().is_some_and(|held| held.0.is_some());
    log_writes(world, held, writes)
}

/// [`logged`] in a packet of its own even inside an open batch, for writes with a layer-1 entry a
/// batch cannot show (a bubble column's water).
pub(crate) fn logged_apart<R>(world: &mut World, writes: impl FnOnce(&mut World) -> R) -> R {
    log_writes(world, false, writes)
}

fn log_writes<R>(world: &mut World, held: bool, writes: impl FnOnce(&mut World) -> R) -> R {
    let opens = !held && !world.get_resource::<Logged>().is_some_and(|log| log.0.is_some());
    if opens {
        world.get_resource_or_insert_with(Logged::default).0 = Some(Vec::new());
    }
    let result = writes(world);
    if opens {
        show_logged(world);
    }
    result
}

/// Keeps a write of `id` into `layer_index` at `pos` where a log is open.
pub(super) fn log(world: &mut World, layer: Entity, pos: BlockPos, layer_index: u32, id: u32) -> bool {
    let Some(log) = world.get_resource_mut::<Logged>().and_then(|log| log.into_inner().0.as_mut()) else { return false };
    log.push((layer, pos, layer_index, id));
    true
}

/// Lists the layer-0 block at `pos` once more where a log is open, for a block BDS writes twice.
pub(crate) fn log_again(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(id) = world.get::<Layer>(layer).map(|level| level.world.block(pos[0], pos[1], pos[2])) else { return };
    log(world, layer, pos, 0, id);
}

/// Lists the layer-1 block at `pos` where a log is open, for water a block took into that layer.
pub(crate) fn log_liquid(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(id) = world.get::<Layer>(layer).map(|level| level.world.liquid(pos[0], pos[1], pos[2])) else { return };
    log(world, layer, pos, 1, id);
}

/// Lists the `flowing_` block of the liquid `id` in layer 1 at `pos` where a log is open, shown but not
/// written: BDS sets a liquid poured beside a block flowing in its tick (bds-bkticks.txt, bds-bubbles.txt).
pub(crate) fn log_flowing(world: &mut World, layer: Entity, pos: BlockPos, id: u32) {
    let Some(registry) = world.get::<Layer>(layer).map(|level| level.world.registry()) else { return };
    let Some(state) = registry.get(id) else { return };
    let flowing = state.name.strip_prefix("minecraft:").and_then(|name| registry.find(&format!("minecraft:flowing_{name}"), &format!("liquid_depth={}", state.liquid_depth)));
    if let Some(flowing) = flowing {
        log(world, layer, pos, 1, flowing);
    }
}

fn show_logged(world: &mut World) {
    let Some(log) = world.get_resource_mut::<Logged>().and_then(|mut log| log.0.take()) else { return };
    let mut sections: Vec<((Entity, BlockPos), Vec<(BlockPos, u32, u32)>)> = Vec::new();
    for (layer, pos, layer_index, id) in log {
        let section = (layer, pos.map(|c| c.div_euclid(16) * 16));
        match sections.iter_mut().find(|(at, _)| *at == section) {
            Some((_, writes)) => writes.push((pos, layer_index, id)),
            None => sections.push((section, vec![(pos, layer_index, id)])),
        }
    }
    for ((layer, corner), writes) in sections {
        let Some(registry) = world.get::<Layer>(layer).map(|level| level.world.registry()) else { continue };
        let packet = match writes.as_slice() {
            [(pos, layer_index, id)] => update_flagged(registry, *pos, *id, *layer_index, FLAGS),
            _ => {
                let of = |wanted: u32| writes.iter().filter(|(_, layer_index, _)| *layer_index == wanted).map(|&(pos, _, id)| update_of(registry, &(pos, id, FLAGS.0))).collect();
                sapling_net::encode(&UpdateSubchunkBlocks { x: corner[0], y: corner[1], z: corner[2], blocks: of(0), extra: of(1) })
            }
        };
        to_viewers(world, layer, corner, &[packet]);
    }
}

/// Cells seen breaking while a batch was open, and the block seen there.
#[derive(Resource, Default)]
pub(crate) struct HeldBreaks(Vec<(Entity, BlockPos, u32)>);

/// Keeps the break of `id` at `pos` to show after the open batch's blocks, if one is open: a
/// `/fill` that clears the cell too shows none, as BDS's (research/captures/bds-n2tnt.txt).
pub(crate) fn hold_break(world: &mut World, layer: Entity, pos: BlockPos, id: u32) -> bool {
    if world.get_resource::<Held>().is_none_or(|held| held.0.is_none()) {
        return false;
    }
    world.get_resource_or_insert_with(HeldBreaks::default).0.push((layer, pos, id));
    true
}

/// Closes the batch and shows what it changed as BDS showed a `/fill` (research/captures/bds-batch.txt):
/// as [`set_sectioned`] does, but each cell as it is now and with the batch's flags (a plugin's: the
/// network flag alone), and then their block entities, which an UpdateBlock would otherwise empty;
/// then the breaks kept by [`hold_break`] of cells that still hold what broke.
pub(crate) fn show_held(world: &mut World) {
    show_held_blocks(world);
    let breaks = world.get_resource_mut::<HeldBreaks>().map(|mut breaks| std::mem::take(&mut breaks.0)).unwrap_or_default();
    for (layer, pos, id) in breaks {
        let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { continue };
        if level.block(pos[0], pos[1], pos[2]) == id {
            let particles = super::destroy_particles(level.registry(), pos, id);
            to_viewers(world, layer, pos, &[particles]);
        }
    }
    let sounds = world.get_resource_mut::<HeldSounds>().map(|mut sounds| std::mem::take(&mut sounds.0)).unwrap_or_default();
    for (layer, pos, sound) in sounds {
        to_viewers(world, layer, pos, &[sound]);
    }
}

/// Sounds kept to play after the open batch's blocks.
#[derive(Resource, Default)]
pub(crate) struct HeldSounds(Vec<(Entity, BlockPos, Bytes)>);

/// Plays `sound` at `pos` now, or after the open batch's blocks, as BDS played dying coral's after
/// their UpdateSubchunkBlocks (research/captures/bds-e3coralmany.txt).
pub(crate) fn after_blocks(world: &mut World, layer: Entity, pos: BlockPos, sound: Bytes) {
    if world.get_resource::<Held>().is_some_and(|held| held.0.is_some()) {
        world.get_resource_or_insert_with(HeldSounds::default).0.push((layer, pos, sound));
    } else {
        to_viewers(world, layer, pos, &[sound]);
    }
}

fn show_held_blocks(world: &mut World) {
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
                // BDS showed no entity with a dispenser or dropper its circuit triggered (bds-z2hand.txt).
                let unshown = ["Dispenser", "Dropper"].map(|id| acacia_proto::nbt::Value::String(id.into()));
                let entity = level.block_entity(pos).filter(|entity| !entity.nbt.get("id").is_some_and(|id| unshown.contains(id)));
                entities.extend(entity.map(|entity| (pos, block_entity_data(pos, entity))));
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

/// Shows the cells a piston moved, as listed (a cell perhaps twice), as BDS did
/// (research/captures/bds-redstone-pistons.txt): an UpdateSubchunkBlocks per sub-chunk, the last
/// written first, each followed by its cells' block entities. `settled` blocks go with the priority
/// flag too and their liquid layer as `extra`; the others' lone cell in a sub-chunk by UpdateBlock.
pub(crate) fn show_moved(world: &mut World, layer: Entity, cells: &[(BlockPos, u32)], settled: bool) {
    let flags = if settled { FLAGS.0 | UpdateBlockFlags::PRIORITY.0 } else { FLAGS.0 };
    let mut sections = by_section(cells.iter().map(|&(pos, id)| (pos, id, flags)));
    sections.reverse();
    let packets: Vec<(BlockPos, Vec<Bytes>)> = {
        let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
        let registry = level.registry();
        sections
            .into_iter()
            .map(|(corner, cells)| {
                let shown = match cells.as_slice() {
                    [(pos, id, flags)] if !settled => update_flagged(registry, *pos, *id, 0, UpdateBlockFlags(*flags)),
                    _ => {
                        let extra = if settled { cells.iter().map(|&([x, y, z], _, flags)| update_of(registry, &([x, y, z], level.liquid(x, y, z), flags))).collect() } else { Vec::new() };
                        sapling_net::encode(&UpdateSubchunkBlocks { x: corner[0], y: corner[1], z: corner[2], blocks: cells.iter().map(|cell| update_of(registry, cell)).collect(), extra })
                    }
                };
                let entities = cells.iter().filter_map(|&(pos, ..)| level.block_entity(pos).map(|entity| block_entity_data(pos, entity)));
                (corner, std::iter::once(shown).chain(entities).collect())
            })
            .collect()
    };
    for (pos, packets) in packets {
        to_viewers(world, layer, pos, &packets);
    }
}

fn update_of(registry: &sapling_world::BlockRegistry, &([x, y, z], id, flags): &Cell) -> BlockUpdate {
    BlockUpdate { position: BlockCoordinates { x, y, z }, runtime_id: sapling_world::block_hash(registry, id), flags, entity_unique_id: i64::MIN, transition_type: BlockUpdateTransitionType::Entity }
}

fn show_sections(world: &mut World, layer: Entity, sections: Vec<(BlockPos, Vec<Cell>)>, single_as_block: bool) {
    let packets: Vec<(BlockPos, Bytes)> = {
        let Some(registry) = world.get::<Layer>(layer).map(|level| level.world.registry()) else { return };
        sections
            .into_iter()
            .map(|(corner, cells)| match cells.as_slice() {
                [(pos, id, flags)] if single_as_block => (*pos, update_flagged(registry, *pos, *id, 0, UpdateBlockFlags(*flags))),
                _ => (corner, sapling_net::encode(&UpdateSubchunkBlocks { x: corner[0], y: corner[1], z: corner[2], blocks: cells.iter().map(|cell| update_of(registry, cell)).collect(), extra: Vec::new() })),
            })
            .collect()
    };
    for (pos, packet) in packets {
        to_viewers(world, layer, pos, &[packet]);
    }
}
