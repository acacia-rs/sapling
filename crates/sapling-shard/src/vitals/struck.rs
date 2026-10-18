//! A mob's hit and a blast on a player: the harm, and the throw BDS sends with it
//! (docs/research/mob-attacks.md, "Hitting" and "Creepers").

use acacia_proto::packets::SetEntityMotion;
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::GameMode;

use super::{Vitals, death, hurt, is_mortal, tell};

use crate::components::{Mode, NetId, Position, Tick};
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
    struck(world, player, amount, kind, speed, Cause::Attack)
}

/// An evoker's fang hurts a player for `amount` by magic and throws it as a hit from the evoker,
/// at `from`, does (research/captures/bds-evoker-findings.md). False where it took nothing.
pub(crate) fn fanged(world: &mut World, player: Entity, amount: f32, kind: &str, from: [f32; 3]) -> bool {
    let Some(feet) = world.get::<Position>(player).map(|position| position.feet) else { return false };
    struck(world, player, amount, kind, away(from, feet), Cause::Magic)
}

fn struck(world: &mut World, player: Entity, amount: f32, kind: &str, speed: [f32; 3], cause: Cause) -> bool {
    let (Some(id), true) = (world.get::<NetId>(player).map(|id| id.0), is_mortal(world, player)) else { return false };
    world.entity_mut(player).insert(death::Slayer(kind.to_owned()));
    // Not one a guard turns away: of two hits at once BDS threw for one (bds-pvp.txt).
    let lands = |vitals: &Vitals| {
        let mut tried = vitals.health;
        tried.hurt(amount) && vitals.health.left() > amount
    };
    if world.get::<Vitals>(player).is_some_and(lands) {
        throw(world, player, id, speed);
    }
    let took = hurt(world, player, amount, cause);
    world.entity_mut(player).remove::<death::Slayer>();
    took
}

/// The throw of a hit from `from` on a player at `feet`: away and up.
fn away(from: [f32; 3], feet: [f32; 3]) -> [f32; 3] {
    let (dx, dz) = (feet[0] - from[0], feet[2] - from[2]);
    let far = dx.hypot(dz).max(1e-4);
    [dx / far * THROW, THROW, dz / far * THROW]
}

/// The player named `by` hits a player for `amount` and throws it at `speed`; one it kills is
/// not thrown. False where it took nothing.
pub(crate) fn punched(world: &mut World, player: Entity, amount: f32, by: &str, speed: [f32; 3]) -> bool {
    bitten(world, player, amount, &format!("{}{by}", death::BY_PLAYER), speed)
}

pub(crate) fn is_fireproof(world: &World, player: Entity) -> bool {
    world.get::<Vitals>(player).is_some_and(|vitals| vitals.health.is_fireproof())
}

/// Sets a player that fire can hurt on fire for `ticks`.
pub(crate) fn set_alight(world: &mut World, player: Entity, ticks: u16) {
    if let Some(mut vitals) = world.get_mut::<Vitals>(player).filter(|vitals| !vitals.health.is_fireproof()) {
        vitals.health.ignite(ticks);
    }
}

/// A blast at `from` hurts a player for `amount` and throws it: as a hit does, and by `push`
/// besides. False where it took nothing.
pub(crate) fn blasted(world: &mut World, player: Entity, amount: f32, from: [f32; 3], push: [f32; 3]) -> bool {
    // BDS's TNT threw a creative player, by the blast alone.
    if let (Some(GameMode::Creative), Some(id)) = (world.get::<Mode>(player).map(|mode| mode.0), world.get::<NetId>(player).map(|id| id.0)) {
        throw(world, player, id, push);
        return false;
    }
    amount > 0.0 && hit_from(world, player, amount, from, push, Cause::EntityExplosion)
}

/// An arrow of a mob of `kind` that flew from `from` hurts a player for `amount` and, if it
/// `throws`, throws it as a hit does. False where it took nothing.
pub(crate) fn shot(world: &mut World, player: Entity, amount: f32, kind: &str, from: [f32; 3], throws: bool) -> bool {
    world.entity_mut(player).insert(death::Slayer(kind.to_owned()));
    let took = match throws {
        true => hit_from(world, player, amount, from, [0.0; 3], Cause::Projectile),
        false => is_mortal(world, player) && hurt(world, player, amount, Cause::Projectile),
    };
    world.entity_mut(player).remove::<death::Slayer>();
    took
}

/// A goat of `kind` at `from` rams a player for `amount`: thrown as a hit is, then again by half
/// that and `push`; one it cannot hurt by `push` alone (research/captures/bds-goat-findings.md).
/// False where it took nothing.
pub(crate) fn rammed(world: &mut World, player: Entity, amount: f32, kind: &str, from: [f32; 3], push: [f32; 3]) -> bool {
    let (Some(id), Some(feet)) = (world.get::<NetId>(player).map(|id| id.0), world.get::<Position>(player).map(|position| position.feet)) else { return false };
    if !is_mortal(world, player) {
        throw(world, player, id, push);
        return false;
    }
    let plain = away(from, feet);
    if world.get::<Vitals>(player).is_some_and(|vitals| vitals.health.left() > amount) {
        throw(world, player, id, plain);
        throw(world, player, id, std::array::from_fn(|axis| plain[axis] / 2.0 + push[axis]));
    }
    world.entity_mut(player).insert(death::Slayer(kind.to_owned()));
    let took = hurt(world, player, amount, Cause::RamAttack);
    world.entity_mut(player).remove::<death::Slayer>();
    took
}

/// Throws a player as a hit from `from` does.
pub(super) fn throw_from(world: &mut World, player: Entity, id: u64, from: [f32; 3]) {
    let Some(feet) = world.get::<Position>(player).map(|position| position.feet) else { return };
    throw(world, player, id, away(from, feet));
}

fn hit_from(world: &mut World, player: Entity, amount: f32, from: [f32; 3], push: [f32; 3], cause: Cause) -> bool {
    let (Some(id), Some(feet), true) = (world.get::<NetId>(player).map(|id| id.0), world.get::<Position>(player).map(|position| position.feet), is_mortal(world, player)) else { return false };
    let (dx, dz) = (feet[0] - from[0], feet[2] - from[2]);
    let far = dx.hypot(dz).max(1e-4);
    if world.get::<Vitals>(player).is_some_and(|vitals| vitals.health.left() > amount) {
        throw(world, player, id, [dx / far * THROW + push[0], THROW + push[1], dz / far * THROW + push[2]]);
    }
    hurt(world, player, amount, cause)
}
