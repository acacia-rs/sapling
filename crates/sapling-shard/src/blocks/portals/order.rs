//! The order a lit or broken Nether portal goes out in, as the Linux BDS's hash containers give it
//! (send/hashed.rs; docs/research/nether-dimension.md, "Section order").

use acacia_proto::types::{BlockCoordinates, BlockUpdate, BlockUpdateTransitionType};
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::shape::{Axis, Shape};
use super::{level, send};

/// A lit frame's cells as BDS writes them: put in an `unordered_set` of 64 buckets along the
/// frame's width (west for an x frame, south for a z one), each column from the bottom up, and
/// taken out in its order.
pub(super) fn lit(shape: &Shape) -> Vec<BlockPos> {
    let mut cells = shape.cells();
    cells.sort_by_key(|&[x, y, z]| match shape.axis {
        Axis::X => (-x, y),
        Axis::Z => (z, y),
    });
    let mut set = send::hashed::Ordered::with_buckets(64);
    for cell in cells {
        set.insert(cell);
    }
    set.into_order()
}

/// Shows `cells` (where, block, flags), already written, one UpdateSubchunkBlocks a sub-chunk in
/// the order BDS's map of the tick's changes gives.
pub(super) fn show(world: &mut World, layer: Entity, cells: &[(BlockPos, u32, u32)]) {
    let Some(registry) = level(world, layer).map(|level| level.registry()) else { return };
    let updates = cells.iter().map(|&([x, y, z], id, flags)| {
        let update = BlockUpdate { position: BlockCoordinates { x, y, z }, runtime_id: registry.network_hash(id), flags, entity_unique_id: 0, transition_type: BlockUpdateTransitionType::Entity };
        ([x, y, z], update)
    });
    let sections = send::hashed::by_section(updates).into_iter().map(|(corner, cells)| (corner.map(|c| c >> 4), cells.into_iter().map(|(_, update)| update).collect()));
    super::show(world, layer, sections.collect());
}

/// Cells written beside a portal their write breaks, held to go out with the sheet, as BDS sent a
/// frame cell set to air inside the sheet's batch (research/captures/bds-n13sheet.txt).
#[derive(Resource, Default)]
pub(super) struct Deferred(Vec<(Entity, BlockPos)>);

/// Whether the cell just written at `pos` is held for the sheet it breaks ([`take`]).
pub(crate) fn defer(world: &mut World, layer: Entity, pos: BlockPos) -> bool {
    let touched = world.get_resource::<super::Touched>().is_some_and(|touched| touched.0.contains(&(layer, pos)));
    if !touched || !super::breaks_beside(world, layer, pos) {
        return false;
    }
    world.get_resource_or_insert_with(Deferred::default).0.push((layer, pos));
    true
}

/// The held cell at `pos`, if held, no longer held.
pub(super) fn take(world: &mut World, layer: Entity, pos: BlockPos) -> bool {
    let Some(mut deferred) = world.get_resource_mut::<Deferred>() else { return false };
    let held = deferred.0.len();
    deferred.0.retain(|cell| *cell != (layer, pos));
    deferred.0.len() != held
}
