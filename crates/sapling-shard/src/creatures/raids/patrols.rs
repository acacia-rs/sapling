//! Pillager patrols, as the pillager's `patrol_captain` and `patrol_follower` groups move them
//! (docs/research/structure-spawns.md "Patrols"): the captain makes for a random block up to
//! [`WANDERS`] off on each axis until it is [`ARRIVED`] near, then picks another
//! (`behavior.move_to_random_block`); a follower makes for the nearest captain within [`LEADS`]
//! until [`FOLLOWS`] near (`behavior.follow_target_captain`), and strolls there.
// TODO: how BDS's MoveToRandomBlockGoal picks its block is not decompiled; a uniform square is assumed.
// TODO: patrol groups are not kept with the world; a reloaded patrol strolls.

use bevy_ecs::prelude::*;

use super::super::{captains::Captain, hunting};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::InLayer;

const CAPTAIN: &str = "minecraft:promote_to_patrol_captain";
const FOLLOWER: &str = "minecraft:spawn_as_patrol_follower";
/// Events that take a pillager out of its patrol (`promote_to_illager_captain` drops `patrol_follower`).
const LEAVES: &str = "minecraft:promote_to_illager_captain";
const WANDERS: f32 = 512.0;
const ARRIVED: f32 = 8.0;
const LEADS: f32 = 64.0;
const FOLLOWS: f32 = 5.0;
/// The pillager's `minecraft:movement` 0.35 times each goal's `speed_multiplier`.
const CAPTAIN_SPEED: f32 = 0.35 * 0.55;
const FOLLOWER_SPEED: f32 = 0.35 * 0.8;

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(in crate::creatures) enum Patrol {
    Captain { goal: Option<[f32; 3]> },
    Follower,
}

fn flat(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// A pillager spawned with `event` joins a patrol as its captain or a follower, or leaves one.
pub(in crate::creatures) fn enlist(world: &mut World, entity: Entity, kind: &str, event: Option<&str>) {
    if kind != "minecraft:pillager" {
        return;
    }
    match event {
        Some(CAPTAIN) => _ = world.entity_mut(entity).insert(Patrol::Captain { goal: None }),
        Some(FOLLOWER) => _ = world.entity_mut(entity).insert(Patrol::Follower),
        Some(LEAVES) => _ = world.entity_mut(entity).remove::<Patrol>(),
        _ => {}
    }
}

/// Before a patrolling pillager's move, when it hunts nobody; false for one in no patrol, and for
/// a follower already by its captain or without one.
pub(in crate::creatures) fn steer(world: &mut World, entity: Entity, layer: Entity) -> bool {
    let (Some(patrol), Some(body)) = (world.get::<Patrol>(entity).copied(), world.get::<Body>(entity).copied()) else { return false };
    match patrol {
        Patrol::Captain { goal } => {
            let goal = goal.filter(|goal| flat(*goal, body.at) > ARRIVED).unwrap_or_else(|| {
                let dice = &mut world.get_resource_or_insert_with(Dice::default);
                let [dx, dz] = [dice.fraction(), dice.fraction()].map(|roll| (roll * 2.0 - 1.0) * WANDERS);
                [body.at[0] + dx, body.at[1], body.at[2] + dz]
            });
            world.entity_mut(entity).insert(Patrol::Captain { goal: Some(goal) });
            hunting::approach(world, entity, layer, &body, goal, CAPTAIN_SPEED, ARRIVED);
            true
        }
        Patrol::Follower => {
            let Some(captain) = captain_near(world, layer, body.at) else { return false };
            if flat(captain, body.at) <= FOLLOWS {
                return false;
            }
            hunting::approach(world, entity, layer, &body, captain, FOLLOWER_SPEED, FOLLOWS);
            true
        }
    }
}

fn captain_near(world: &mut World, layer: Entity, at: [f32; 3]) -> Option<[f32; 3]> {
    let mut captains = world.query_filtered::<(&InLayer, &Body, &Patrol), With<Captain>>();
    captains
        .iter(world)
        .filter(|(on, body, patrol)| on.0 == layer && matches!(patrol, Patrol::Captain { .. }) && flat(body.at, at) <= LEADS)
        .map(|(_, body, _)| body.at)
        .min_by(|a, b| flat(*a, at).total_cmp(&flat(*b, at)))
}
