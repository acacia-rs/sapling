//! Boats and minecarts in fire or lava: hurt each tick until their integrity is gone, then broken a
//! tick later into their item, as BDS's were (research/captures/bds-ffburn.txt). A boat in lava took
//! 4 (all of its 40 at once); one in fire, and a cart in fire or resting on lava, 2 a tick: 3 ticks.
//! BDS's boat in fire lost 1 on its third tick, where this takes 2; it breaks the same tick.
//! A TNT minecart is primed instead (research/captures/bds-fftnt.txt).

use bevy_ecs::prelude::*;

use super::{TNT_MINECART, Vehicle, carts, shake, wreck};
use crate::body::Body;
use crate::components::{InLayer, Layer};
use crate::creatures::touches;
use crate::entities::Kind;

const LAVA: &[&str] = &["minecraft:lava", "minecraft:flowing_lava"];
const FIERY: &[&str] = &["minecraft:lava", "minecraft:flowing_lava", "minecraft:fire", "minecraft:soul_fire"];
const IN_LAVA: f32 = 4.0;
const SCORCHED: f32 = 2.0;
/// How far under its box a vehicle feels lava or fire: BDS's carts burned resting on lava.
const BELOW: f32 = 0.01;

/// A vehicle burnt through, to break next tick.
#[derive(Component)]
pub(super) struct BurntThrough;

pub(super) fn tick(world: &mut World) {
    let burnt: Vec<Entity> = world.query_filtered::<Entity, With<BurntThrough>>().iter(world).collect();
    for vehicle in burnt {
        wreck(world, vehicle, true);
    }
    let vehicles: Vec<(Entity, Entity, Body)> = world.query_filtered::<(Entity, &InLayer, &Body), With<Vehicle>>().iter(world).map(|(vehicle, layer, body)| (vehicle, layer.0, *body)).collect();
    for (vehicle, layer, body) in vehicles {
        let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { continue };
        let mut lower = body;
        lower.at[1] -= BELOW;
        // Only boats were seen in lava: BDS's carts stayed on top of it, 2 a tick.
        let is_boat = world.get::<Kind>(vehicle).is_some_and(|kind| super::is_boat(&kind.0));
        let damage = match (is_boat && touches(level, &body, LAVA), touches(level, &lower, FIERY)) {
            (true, _) => IN_LAVA,
            (false, true) => SCORCHED,
            (false, false) => continue,
        };
        if world.get::<Kind>(vehicle).is_some_and(|kind| kind.0 == TNT_MINECART) {
            carts::prime_burning(world, vehicle);
            continue;
        }
        if shake(world, vehicle, damage).is_some_and(|left| left <= 0) {
            world.entity_mut(vehicle).insert(BurntThrough);
        }
    }
}
