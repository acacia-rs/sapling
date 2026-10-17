//! Sounds BDS plays a few ticks after what makes them.

use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::BlockPos;

use crate::blocks;
use crate::components::Tick;

/// Sounds due later: when, the world, where, the sound.
#[derive(Resource, Default)]
pub(crate) struct LateSounds(Vec<(u64, Entity, BlockPos, Bytes)>);

/// Has those who see `pos` hear `sound` `after` ticks from now; at once for 0.
pub(crate) fn play_later(world: &mut World, after: u64, layer: Entity, pos: BlockPos, sound: Bytes) {
    if after == 0 {
        return blocks::to_viewers(world, layer, pos, &[sound]);
    }
    let due = world.resource::<Tick>().0 + after;
    world.get_resource_or_insert_with(LateSounds::default).0.push((due, layer, pos, sound));
}

/// Plays the sounds whose time has come.
pub(crate) fn play_due(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let Some(mut late) = world.get_resource_mut::<LateSounds>() else { return };
    let (due, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut late.0).into_iter().partition(|(when, ..)| *when <= now);
    late.0 = waiting;
    for (_, layer, pos, sound) in due {
        blocks::to_viewers(world, layer, pos, &[sound]);
    }
}
