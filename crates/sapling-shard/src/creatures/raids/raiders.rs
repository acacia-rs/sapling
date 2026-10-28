//! What a raid's raiders do when they hunt nobody, as their pack's `raid_configuration` and
//! `celebrate` groups have them: make for the village's middle (`behavior.move_to_village`,
//! `goal_radius` 2, at their movement times its `speed_multiplier`), and once the village has
//! fallen celebrate (`behavior.celebrate`: 30 s, a hop every 1 to 3.5 s and `celebrate` heard
//! every 2 to 7 s; BDS fires `minecraft:start_celebrating` from `Village`, 0x142a5d640).

use bevy_ecs::prelude::*;

use super::super::{hunting, legs};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{NetId, Tick};
use crate::late_sounds;

/// The village's middle is reached this near.
const GOAL: f32 = 2.0;
/// Each kind's `minecraft:movement` times its `move_to_village` `speed_multiplier`.
const SPEEDS: [(&str, f32); 5] =
    [("minecraft:pillager", 0.35), ("minecraft:vindicator", 0.35), ("minecraft:ravager", 0.3), ("minecraft:evocation_illager", 0.35), ("minecraft:witch", 0.3)];
/// Ticks a celebration lasts, and the least and most between its hops and its cries.
const CELEBRATES: u64 = 600;
const HOPS: [u32; 2] = [20, 70];
const CRIES: [u32; 2] = [40, 140];

/// A mob in a raid on the village about `middle`.
#[derive(Component, Clone, Copy)]
pub(super) struct Raider {
    pub middle: [f32; 3],
}

#[derive(Component, Clone, Copy)]
struct Celebrating {
    until: u64,
    hop: u64,
    cry: u64,
}

fn after(dice: &mut Dice, now: u64, [least, most]: [u32; 2]) -> u64 {
    now + u64::from(least + dice.roll(most - least + 1))
}

/// The raiders of a fallen village start celebrating.
pub(super) fn celebrate(world: &mut World, raiders: &[Entity], now: u64) {
    for &raider in raiders {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        let party = Celebrating { until: now + CELEBRATES, hop: after(&mut dice, now, HOPS), cry: after(&mut dice, now, CRIES) };
        if let Ok(mut raider) = world.get_entity_mut(raider) {
            raider.insert(party);
        }
    }
}

/// Before a raider's move, when it hunts nobody: it celebrates, or makes for the village's middle.
/// False for a mob in no raid.
pub(in crate::creatures) fn steer(world: &mut World, entity: Entity, layer: Entity, kind: &str) -> bool {
    let (Some(raider), Some(body)) = (world.get::<Raider>(entity).copied(), world.get::<Body>(entity).copied()) else { return false };
    let now = world.resource::<Tick>().0;
    if let Some(party) = world.get::<Celebrating>(entity).copied() {
        party_on(world, entity, layer, kind, &body, party, now);
        return true;
    }
    let speed = SPEEDS.iter().find(|(raider, _)| *raider == kind).map_or(0.3, |(_, speed)| *speed);
    hunting::approach(world, entity, layer, &body, raider.middle, speed, GOAL);
    true
}

fn party_on(world: &mut World, entity: Entity, layer: Entity, kind: &str, body: &Body, mut party: Celebrating, now: u64) {
    legs::rest(world, entity);
    if now >= party.until {
        world.entity_mut(entity).remove::<Celebrating>();
        return;
    }
    let mut dice = world.remove_resource::<Dice>().unwrap_or_default();
    if now >= party.hop {
        legs::hop(world, entity);
        party.hop = after(&mut dice, now, HOPS);
    }
    let cries = now >= party.cry;
    if cries {
        party.cry = after(&mut dice, now, CRIES);
    }
    world.insert_resource(dice);
    world.entity_mut(entity).insert(party);
    if let Some(id) = world.get::<NetId>(entity).map(|id| id.0).filter(|_| cries) {
        late_sounds::play_later(world, 0, layer, body.at.map(|c| c.floor() as i32), hunting::sound("celebrate", id, kind, body, hunting::HEAD));
    }
}

/// The raid is over and gone: its raiders are in none.
pub(super) fn disband(world: &mut World, raiders: &[Entity]) {
    for &raider in raiders {
        if let Ok(mut raider) = world.get_entity_mut(raider) {
            raider.remove::<Raider>();
        }
    }
}

/// Whether the mob is a raider celebrating.
#[cfg(test)]
pub(super) fn is_celebrating(world: &World, entity: Entity) -> bool {
    world.get::<Celebrating>(entity).is_some()
}
