//! `/clone`'s writes as BDS's clone (fcn 14160c130) shows them (research/captures/bds-x2clone.txt): a
//! move's emptied source first; each copy's block with its source's layer 1 listed as `extra`, changed
//! or not; every copy listed once more (BDS sets each twice); then each copy's entity shown twice.

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_api::{Block, BlockPos};

use super::send;
use crate::components::Layer;

/// While a clone writes: the cells whose entities were kept unshown, to show after its blocks.
#[derive(Resource, Default)]
pub(crate) struct Cloning(Option<Vec<(Entity, BlockPos)>>);

/// Whether a clone is writing, which then keeps the entity at `pos` to show after its blocks.
pub(super) fn defers_entity(world: &mut World, layer: Entity, pos: BlockPos) -> bool {
    let Some(deferred) = world.get_resource_mut::<Cloning>().and_then(|cloning| cloning.into_inner().0.as_mut()) else { return false };
    if !deferred.contains(&(layer, pos)) {
        deferred.push((layer, pos));
    }
    true
}

/// A block `copy` puts: from where, to where, the block, and the entity it carries (kept by `keep`).
pub(crate) struct Copied<'a> {
    pub(crate) from: BlockPos,
    pub(crate) to: BlockPos,
    pub(crate) block: &'a Block,
    pub(crate) entity: Option<&'a Value>,
}

/// Copies blocks into `layer` as BDS's clone, `cleared` emptied first; `keep` gives a copy's cell the
/// entity it carries. The blocks copied as BDS counts them, a layer-1 block too (bds-x2clone.txt: a
/// waterlogged fence counted 2).
pub(crate) fn copy(world: &mut World, layer: Entity, cleared: &[BlockPos], copies: &[Copied<'_>], keep: impl Fn(&mut World, BlockPos, &Value)) -> u32 {
    let Some(level) = world.get::<Layer>(layer).map(|level| &level.world) else { return 0 };
    let air = level.registry().air_id();
    let liquids: Vec<u32> = copies.iter().map(|copy| level.liquid(copy.from[0], copy.from[1], copy.from[2])).collect();
    let counted = (copies.len() + liquids.iter().filter(|&&liquid| liquid != air).count()) as u32;
    world.get_resource_or_insert_with(Cloning::default).0 = Some(Vec::new());
    send::logged(world, |world| {
        // Settled once all are written, as a `/fill`'s (updates.rs `Unsettled`; bds-al2fill.txt).
        let keeps = super::updates::keep_unsettled(world);
        for &pos in cleared {
            super::update_block(world, layer, pos, &Block::air());
        }
        for (copy, &liquid) in copies.iter().zip(&liquids) {
            super::update_block(world, layer, copy.to, copy.block);
            if let Some(entity) = copy.entity {
                keep(world, copy.to, &crate::spawners::cloned(entity));
            }
            if !send::set_liquid(world, layer, copy.to, liquid) {
                send::log_liquid(world, layer, copy.to);
            }
        }
        for copy in copies {
            send::log_again(world, layer, copy.to);
        }
        if keeps {
            super::updates::settle_kept(world);
        }
    });
    let deferred = world.get_resource_mut::<Cloning>().and_then(|mut cloning| cloning.0.take()).unwrap_or_default();
    for (at, pos) in deferred {
        let times = if at == layer && copies.iter().any(|copy| copy.to == pos && copy.entity.is_some()) { 2 } else { 1 };
        let Some(shown) = world.get::<Layer>(at).and_then(|level| level.world.block_entity(pos).map(|entity| send::block_entity_data(pos, entity))) else { continue };
        send::to_viewers(world, at, pos, &vec![shown; times]);
    }
    counted
}
