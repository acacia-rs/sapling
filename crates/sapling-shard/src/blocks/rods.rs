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
/// The sparks BDS showed at a struck rod's corner, before powering it (research/captures/bds-rsdevices.txt).
const SPARKS: usize = 6;

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

/// The hook for lightning striking the cell `pos` (the block under the bolt): a rod there is
/// powered, and copper there and from the block below or the rod's own block on is cleaned
/// (docs/research/copper.md, L3-L4). Whether it was a rod.
pub(crate) fn struck(world: &mut World, layer: Entity, pos: BlockPos) -> bool {
    if is_rod(world, layer, pos).is_some() {
        let position = Vec3f { x: pos[0] as f32, y: pos[1] as f32, z: pos[2] as f32 };
        let spark = sapling_net::encode(&LevelEvent { event: LevelEventEvent::ParticleElectricSpark, position, data: 0 });
        send::to_viewers(world, layer, pos, &vec![spark; SPARKS]);
    }
    let is_rod = power(world, layer, pos, true);
    let start = match is_rod.then(|| facing(world, layer, pos)).flatten() {
        Some(face) => super::opposite(face).neighbour(pos),
        None => Face::Down.neighbour(pos),
    };
    if is_rod {
        queued::cancel(world, layer, pos, Job::RodOff);
        queued::after(world, layer, pos, POWERED, Job::RodOff);
    }
    super::copper::struck(world, layer, pos, start);
    super::vibrate_at(world, layer, pos, super::GameEvent::LightningStrike, false);
    is_rod
}

fn facing(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Option<Face> {
    let level = level(world, layer)?;
    let state = level.registry().get(level.block(x, y, z))?;
    state.properties.split(',').find_map(|pair| pair.strip_prefix("facing_direction="))?.parse().ok().and_then(Face::from_index)
}

pub(super) fn ended(world: &mut World, layer: Entity, pos: BlockPos) {
    power(world, layer, pos, false);
}
