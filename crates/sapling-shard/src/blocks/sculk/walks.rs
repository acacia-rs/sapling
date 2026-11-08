//! A player's steps, as vibrations.

use bevy_ecs::prelude::*;

use super::{GameEvent, Listeners, is_sensor, name_at, reach_listeners, vibrate_at_by};

/// A step's share of a block walked (the Java game's `moveDist` of 0.6 a block, a step at each whole).
const STRIDE: f32 = 0.6;

/// A player walked from `from` to `to` (feet): a step is heard every 1/0.6 blocks walked on the
/// ground, but not sneaking; a sensor walked on is touched, sneaking or not (S14).
pub(crate) fn walked(world: &mut World, player: Entity, layer: Entity, from: [f32; 3], to: [f32; 3], on_ground: bool, sneaking: bool) {
    if !on_ground {
        return;
    }
    let under = [to[0].floor() as i32, (to[1] - 0.5).floor() as i32, to[2].floor() as i32];
    if name_at(world, layer, under).is_some_and(|(name, ..)| is_sensor(name)) {
        vibrate_at_by(world, layer, under, GameEvent::SculkTouch, player);
    }
    if sneaking {
        return;
    }
    let walked = (to[0] - from[0]).hypot(to[2] - from[2]) * STRIDE;
    let mut listeners = world.get_resource_or_insert_with(Listeners::default);
    let gone = listeners.strides.entry(player).or_insert(0.0);
    *gone += walked;
    let stepped = *gone >= 1.0;
    *gone = gone.fract();
    if stepped {
        crate::creatures::warden_hears(world, layer, to, GameEvent::Step, Some(player));
        reach_listeners(world, layer, to, GameEvent::Step, true);
    }
}
