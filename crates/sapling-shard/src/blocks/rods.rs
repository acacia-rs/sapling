//! Lightning rods as redstone sources: a strike powers one for a while (docs/research/redstone-
//! blocks.md, "Lightning rod"). The strike itself is the lightning's (`struck`).

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use super::queued::{self, Job};
use super::{level, send, with_property};

/// Ticks a struck rod stays powered.
const POWERED: u64 = 8;

fn is_rod(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Option<u32> {
    let level = level(world, layer).filter(|level| level.in_height(y))?;
    let id = level.block(x, y, z);
    level.registry().get(id).filter(|state| state.name.ends_with("lightning_rod")).map(|_| id)
}

fn power(world: &mut World, layer: Entity, pos: BlockPos, on: bool) -> bool {
    let Some(id) = is_rod(world, layer, pos) else { return false };
    let Some(powered) = level(world, layer).and_then(|level| with_property(level.registry(), id, "powered_bit", if on { "1" } else { "0" })) else { return false };
    send::set(world, layer, pos, powered);
    true
}

/// The cells lightning's hook is to run on for a bolt on `pos` (copper.rs `walked`), from the block under
/// it or, for a rod, the one it stands on; their sparks are shown now, as the bolt appears, its strike
/// coming a tick later for a summoned one (research/captures/bds-rsdevices.txt, bds-p2copper.txt).
pub(crate) fn walk(world: &mut World, layer: Entity, pos: BlockPos) -> Vec<BlockPos> {
    let start = match is_rod(world, layer, pos).and_then(|_| facing(world, layer, pos)) {
        Some(face) => super::opposite(face).neighbour(pos),
        None => Face::Down.neighbour(pos),
    };
    let visits = super::copper::walked(world, layer, pos, start);
    let sparks: Vec<_> = super::copper::sparking(world, layer, &visits).into_iter().map(super::copper::spark_at).collect();
    if !sparks.is_empty() {
        send::to_viewers(world, layer, pos, &sparks);
    }
    visits
}

/// Lightning's hook on each of `visits` ([`walk`]), the bolt's own cell first: a rod is powered, copper
/// loses a stage (docs/research/copper.md, L1-L5).
pub(crate) fn struck(world: &mut World, layer: Entity, visits: &[BlockPos]) {
    for &pos in visits {
        if power(world, layer, pos, true) {
            queued::cancel(world, layer, pos, Job::RodOff);
            queued::after(world, layer, pos, POWERED, Job::RodOff);
        }
        super::copper::clean(world, layer, pos);
    }
    if let Some(&pos) = visits.first() {
        super::vibrate_at(world, layer, pos, super::GameEvent::LightningStrike, false);
    }
}

fn facing(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Option<Face> {
    let level = level(world, layer)?;
    let state = level.registry().get(level.block(x, y, z))?;
    state.properties.split(',').find_map(|pair| pair.strip_prefix("facing_direction="))?.parse().ok().and_then(Face::from_index)
}

pub(super) fn ended(world: &mut World, layer: Entity, pos: BlockPos) {
    power(world, layer, pos, false);
}
