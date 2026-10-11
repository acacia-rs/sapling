//! A mob's hit and a blast on a player: the harm, and the throw BDS sends with it
//! (docs/research/mob-attacks.md, "Hitting" and "Creepers").

use acacia_proto::packets::SetEntityMotion;
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;

use super::{Vitals, death, hurt, is_mortal, tell};
use crate::components::{NetId, Position, Tick};
use crate::creatures::Cause;

/// The speed a hit throws a player at, away from what hit it and up.
const THROW: f32 = 0.4;

fn throw(world: &mut World, player: Entity, id: u64, [x, y, z]: [f32; 3]) {
    let tick = world.resource::<Tick>().0;
    tell(world, player, &SetEntityMotion { runtime_entity_id: id, velocity: Vec3f { x, y, z }, tick });
}

/// A mob of `kind` hits a player for `amount` and throws it at `speed`; one it kills is not
/// thrown (research/captures/bds-slain.txt). False where it took nothing.
pub(crate) fn bitten(world: &mut World, player: Entity, amount: f32, kind: &str, speed: [f32; 3]) -> bool {
    let (Some(id), true) = (world.get::<NetId>(player).map(|id| id.0), is_mortal(world, player)) else { return false };
    world.entity_mut(player).insert(death::Slayer(kind.to_owned()));
    if world.get::<Vitals>(player).is_some_and(|vitals| vitals.health.left() > amount) {
        throw(world, player, id, speed);
    }
    let took = hurt(world, player, amount, Cause::Attack);
    world.entity_mut(player).remove::<death::Slayer>();
    took
}

/// A blast at `from` hurts a player for `amount` and throws it: as a hit does, and by `push`
/// besides. False where it took nothing.
pub(crate) fn blasted(world: &mut World, player: Entity, amount: f32, from: [f32; 3], push: [f32; 3]) -> bool {
    let (Some(id), Some(feet), true) = (world.get::<NetId>(player).map(|id| id.0), world.get::<Position>(player).map(|position| position.feet), is_mortal(world, player)) else { return false };
    let (dx, dz) = (feet[0] - from[0], feet[2] - from[2]);
    let far = dx.hypot(dz).max(1e-4);
    if world.get::<Vitals>(player).is_some_and(|vitals| vitals.health.left() > amount) {
        throw(world, player, id, [dx / far * THROW + push[0], THROW + push[1], dz / far * THROW + push[2]]);
    }
    hurt(world, player, amount, Cause::EntityExplosion)
}
