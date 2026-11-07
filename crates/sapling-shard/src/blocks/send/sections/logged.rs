//! Writes kept while [`logged`] runs and shown as BDS shows a sub-chunk's writes of a tick.

use acacia_proto::packets::UpdateSubchunkBlocks;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::super::{FLAGS, to_viewers, update_flagged};
use super::{Held, play_held_sounds, update_of};
use crate::components::Layer;

/// The writes kept while [`logged`] runs: each one's world, cell, layer (0 or 1) and block written.
#[derive(Resource, Default)]
pub(crate) struct Logged(Option<Vec<(Entity, BlockPos, u32, u32)>>);

/// Runs `writes` keeping every block [`super::super::set`] and [`super::super::set_liquid`] write, then
/// shows them as BDS shows a sub-chunk's writes of a tick (bds-plantpop2.txt, bds-fixset.txt): each
/// write listed in the order made, layer 1's as `extra`, in one UpdateSubchunkBlocks, or an UpdateBlock
/// for a write alone. Inside an open batch or log, that takes the writes instead.
pub(crate) fn logged<R>(world: &mut World, writes: impl FnOnce(&mut World) -> R) -> R {
    let held = world.get_resource::<Held>().is_some_and(|held| held.0.is_some());
    log_writes(world, held, writes)
}

/// [`logged`], but inside an open batch each write goes into it as made, with the block it wrote, where
/// the batch otherwise lists a cell once as it ends: BDS's `/fill` of rails lists every write
/// (research/captures/bds-ao2rails.txt). A layer-1 write goes in as its cell.
pub(crate) fn listed<R>(world: &mut World, writes: impl FnOnce(&mut World) -> R) -> R {
    let held = world.get_resource::<Held>().is_some_and(|held| held.0.is_some());
    if !held || is_open(world) {
        return logged(world, writes);
    }
    world.get_resource_or_insert_with(Logged::default).0 = Some(Vec::new());
    let result = writes(world);
    let log = world.get_resource_mut::<Logged>().and_then(|mut log| log.0.take()).unwrap_or_default();
    if let Some((_, cells)) = world.get_resource_mut::<Held>().and_then(|held| held.into_inner().0.as_mut()) {
        cells.extend(log.into_iter().map(|(layer, pos, layer_index, id)| (layer, pos, (layer_index == 0).then_some(id))));
    }
    result
}

/// [`logged`] in a packet of its own even inside an open batch, for writes with a layer-1 entry a
/// batch cannot show (a bubble column's water).
pub(crate) fn logged_apart<R>(world: &mut World, writes: impl FnOnce(&mut World) -> R) -> R {
    log_writes(world, false, writes)
}

fn log_writes<R>(world: &mut World, held: bool, writes: impl FnOnce(&mut World) -> R) -> R {
    let opens = !held && !is_open(world);
    if opens {
        world.get_resource_or_insert_with(Logged::default).0 = Some(Vec::new());
    }
    let result = writes(world);
    if opens {
        show_logged(world);
        if world.get_resource::<Held>().is_none_or(|held| held.0.is_none()) {
            play_held_sounds(world);
        }
    }
    result
}

pub(super) fn is_open(world: &World) -> bool {
    world.get_resource::<Logged>().is_some_and(|log| log.0.is_some())
}

/// Keeps a write of `id` into `layer_index` at `pos` where a log is open.
pub(crate) fn log(world: &mut World, layer: Entity, pos: BlockPos, layer_index: u32, id: u32) -> bool {
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
