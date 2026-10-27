//! Squids and glow squids, as BDS's drifted in the `swimmers` drill
//! (research/captures/bds-wander-findings.md): in water each is pushed 0.00293 a tick along a way
//! it keeps for about ten seconds, water keeping 0.8 of its speed (`Physics::SWIMMER`), so it
//! drifts at 0.0146 a tick.
// TODO: how long a way is kept and how steep one may be are guessed from the two seen; a squid's
// spurts of 0.08 every 12 ticks (`squid_flee`?), its ink and its going down to the floor
// (`squid_dive`) are not copied, and out of water it does not flop.

use bevy_ecs::prelude::*;

use crate::blocks::Dice;
use crate::body::{Body, Liquid};
use crate::sync::Told;

const KINDS: &[&str] = &["minecraft:squid", "minecraft:glow_squid"];
const PUSH: f32 = 0.00293;
/// A tick in this many it takes another way; a way rises or falls up to this much a block across.
const ODDS: u32 = 200;
const STEEP: f32 = 0.5;

#[derive(Component, Default)]
pub(super) struct Squid {
    way: Option<[f32; 3]>,
}

pub(super) fn arm(world: &mut World, entity: Entity, kind: &str) {
    if KINDS.contains(&kind) {
        world.entity_mut(entity).insert(Squid::default());
    }
}

/// Before its move: in water a squid drifts on its way, now and then another.
pub(super) fn steer(world: &mut World, entity: Entity) -> bool {
    let (Some(body), Some(way)) = (world.get::<Body>(entity).copied(), world.get::<Squid>(entity).map(|squid| squid.way)) else { return false };
    if body.liquid() != Some(Liquid::Water) {
        return true;
    }
    let mut dice = world.get_resource_or_insert_with(Dice::default);
    let way = match way.filter(|_| dice.roll(ODDS) != 0) {
        Some(way) => way,
        None => {
            let (yaw, rise) = (dice.fraction() * std::f32::consts::TAU, (dice.fraction() * 2.0 - 1.0) * STEEP);
            let long = (1.0 + rise * rise).sqrt();
            [-yaw.sin() / long, rise / long, yaw.cos() / long]
        }
    };
    if let Some(mut squid) = world.get_mut::<Squid>(entity) {
        squid.way = Some(way);
    }
    if let Some(mut moving) = world.get_mut::<Body>(entity) {
        moving.fly(way.map(|c| c * PUSH));
    }
    let yaw = (-way[0]).atan2(way[2]).to_degrees();
    if let Some(mut told) = world.get_mut::<Told>(entity) {
        told.face(yaw, yaw);
    }
    true
}
