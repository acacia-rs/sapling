//! A punch on a face with fire over it puts the fire out instead of starting a break, as
//! dragonfly's `Player.StartBreaking` and the Java game's `Level.extinguishFire`: the fire goes
//! with `extinguish.fire` heard.

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use super::{level, send};

const FIRES: [&str; 2] = ["minecraft:fire", "minecraft:soul_fire"];

/// Puts out a fire on `face` of the block at `pos` that `player` punched; whether there was one.
pub(super) fn put_out(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, face: Face) -> bool {
    let at = face.neighbour(pos);
    let Some(level) = level(world, layer).filter(|level| level.in_height(at[1])) else { return false };
    let (registry, id) = (level.registry(), level.block(at[0], at[1], at[2]));
    if !registry.get(id).is_some_and(|state| FIRES.contains(&state.name)) {
        return false;
    }
    let (sound, air) = (send::sound("extinguish.fire", registry, at, id), registry.air_id());
    if !crate::hand_events::may_extinguish(world, player, at) {
        send::resync(world, player, &[at]);
        return true;
    }
    send::set(world, layer, at, air);
    send::to_viewers(world, layer, at, &[sound]);
    true
}
