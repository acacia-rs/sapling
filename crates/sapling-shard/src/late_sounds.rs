//! Sounds BDS plays a few ticks after what makes them.

use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::BlockPos;

use crate::components::Tick;
use crate::view::to_viewers_but;

/// Sounds due later: when, the world, where, who does not hear it, the sound.
#[derive(Resource, Default)]
pub(crate) struct LateSounds(Vec<(u64, Entity, BlockPos, Option<Entity>, Bytes)>);

/// Has those who see `pos` hear `sound` `after` ticks from now; at once for 0.
pub(crate) fn play_later(world: &mut World, after: u64, layer: Entity, pos: BlockPos, sound: Bytes) {
    play_later_but(world, after, layer, pos, None, sound);
}

/// [`play_later`], but for the player `but`.
pub(crate) fn play_later_but(world: &mut World, after: u64, layer: Entity, pos: BlockPos, but: Option<Entity>, sound: Bytes) {
    if after == 0 {
        return to_viewers_but(world, layer, pos, but, &[sound]);
    }
    let due = world.resource::<Tick>().0 + after;
    world.get_resource_or_insert_with(LateSounds::default).0.push((due, layer, pos, but, sound));
}

/// Plays the sounds whose time has come.
pub(crate) fn play_due(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let Some(mut late) = world.get_resource_mut::<LateSounds>() else { return };
    let (due, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut late.0).into_iter().partition(|(when, ..)| *when <= now);
    late.0 = waiting;
    for (_, layer, pos, but, sound) in due {
        to_viewers_but(world, layer, pos, but, &[sound]);
    }
}
