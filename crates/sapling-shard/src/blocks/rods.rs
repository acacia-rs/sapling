//! Lightning rods as redstone sources: a strike powers one for a while (docs/research/redstone-
//! blocks.md, "Lightning rod"). The strike itself is the lightning's (`struck`).

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use super::queued::{self, Job};
use super::{level, send, with_property};

/// Ticks a struck rod stays powered.
const POWERED: u64 = 8;

/// What a bolt does to the blocks about it, rolled as it appears: the cells its hook calls reach,
/// one for each call, and the cell its copper walk starts from.
#[derive(Clone, Default)]
pub(crate) struct Strike {
    hooks: Vec<BlockPos>,
    start: BlockPos,
}

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

/// A bolt appears in `cell`: its hook calls are rolled, each showing a spark at its cell's corner
/// now; what they do comes with its strike, a tick later for a summoned one ([`struck`]). A bolt in a
/// rod's cell sparks once, one over a rod 3 to 7 times, its walk's calls on the rod (bds-o3rod.txt).
pub(crate) fn sparks(world: &mut World, layer: Entity, cell: BlockPos) -> Strike {
    let start = match is_rod(world, layer, cell).and_then(|_| facing(world, layer, cell)) {
        Some(face) => super::opposite(face).neighbour(cell),
        None => Face::Down.neighbour(cell),
    };
    let hooks = super::copper::hooks(world, layer, cell, start);
    let shown: Vec<bytes::Bytes> = hooks
        .iter()
        .map(|pos| sapling_net::encode(&LevelEvent { event: LevelEventEvent::ParticleElectricSpark, position: Vec3f { x: pos[0] as f32, y: pos[1] as f32, z: pos[2] as f32 }, data: 0 }))
        .collect();
    if !shown.is_empty() {
        send::to_viewers(world, layer, cell, &shown);
    }
    Strike { hooks, start }
}

/// The strike of a bolt whose hook calls `strike` rolled: copper they reach loses its stages and
/// rods are powered for a while. Every change is listed in the order made, a sub-chunk at a time,
/// as BDS showed them (bds-p2copper.txt).
pub(crate) fn struck(world: &mut World, layer: Entity, strike: &Strike) {
    send::logged(world, |world| {
        for &pos in &strike.hooks {
            if super::copper::hooked(world, layer, pos) && power(world, layer, pos, true) {
                queued::cancel(world, layer, pos, Job::RodOff);
                queued::after(world, layer, pos, POWERED, Job::RodOff);
            }
        }
    });
}

/// The strike is felt from its walk's start (copper.md L6): the cell under a bolt, or the cell a
/// struck rod points to, as BDS's vibrations came from (bds-z2bolt.txt).
pub(crate) fn felt(world: &mut World, layer: Entity, strike: &Strike) {
    super::vibrate_at(world, layer, strike.start, super::GameEvent::LightningStrike, false);
}

fn facing(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Option<Face> {
    let level = level(world, layer)?;
    let state = level.registry().get(level.block(x, y, z))?;
    state.properties.split(',').find_map(|pair| pair.strip_prefix("facing_direction="))?.parse().ok().and_then(Face::from_index)
}

pub(super) fn ended(world: &mut World, layer: Entity, pos: BlockPos) {
    power(world, layer, pos, false);
}
