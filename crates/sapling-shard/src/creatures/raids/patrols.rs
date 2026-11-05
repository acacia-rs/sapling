//! Pillager patrols, as the pillager's `patrol_captain` and `patrol_follower` groups move them
//! (docs/research/structure-spawns.md "Patrols"): the captain makes for a random block up to
//! [`WANDERS`] off on each axis until it is [`ARRIVED`] near, then picks another
//! (`behavior.move_to_random_block`, 148838db0), [`LEG`] at a time (1488393e0), each leg cornerwise
//! until level with its end on one axis, then straight, as BDS's paths went; a follower makes for
//! the nearest captain within [`LEADS`] until [`FOLLOWS`] near (`behavior.follow_target_captain`),
//! and strolls there. A kept pillager keeps its group in its `definitions`, as BDS's.
// TODO: a stalled leg's sidestep and the 400-tick rest after four (1488393e0) are not ported.

use bevy_ecs::prelude::*;

use super::super::{captains::Captain, hunting};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::InLayer;
use crate::entities::{self, Stored};

const CAPTAIN: &str = "minecraft:promote_to_patrol_captain";
const FOLLOWER: &str = "minecraft:spawn_as_patrol_follower";
/// Events that take a pillager out of its patrol (`promote_to_illager_captain` drops `patrol_follower`).
const LEAVES: &str = "minecraft:promote_to_illager_captain";
/// The pack's component groups of a captain and of a follower, as `definitions` lists them.
const GROUPS: [&str; 2] = ["patrol_captain", "patrol_follower"];
const WANDERS: f32 = 512.0;
const ARRIVED: f32 = 8.0;
/// Half the pillager's follow range (64): one leg of the captain's walk, ended on a whole cell.
const LEG: f32 = 32.0;
const LEADS: f32 = 64.0;
const FOLLOWS: f32 = 5.0;
/// The pillager's `minecraft:movement` 0.35 times each goal's `speed_multiplier`.
const CAPTAIN_SPEED: f32 = 0.35 * 0.55;
const FOLLOWER_SPEED: f32 = 0.35 * 0.8;

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(in crate::creatures) enum Patrol {
    Captain { goal: Option<[f32; 3]>, leg: Option<[f32; 3]> },
    Follower,
}

const NEW_CAPTAIN: Patrol = Patrol::Captain { goal: None, leg: None };

fn flat(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// A pillager spawned with `event` joins a patrol as its captain or a follower, or leaves one.
pub(in crate::creatures) fn enlist(world: &mut World, entity: Entity, kind: &str, event: Option<&str>) {
    if kind != "minecraft:pillager" {
        return;
    }
    match event {
        Some(CAPTAIN) => _ = world.entity_mut(entity).insert(NEW_CAPTAIN),
        Some(FOLLOWER) => _ = world.entity_mut(entity).insert(Patrol::Follower),
        Some(LEAVES) => {
            world.entity_mut(entity).remove::<Patrol>();
            entities::restore(world, entity, |compound, _| sapling_world::regrouped_actor(compound, &GROUPS, &[]).ok());
        }
        _ => {}
    }
}

/// Writes each kept patrolling pillager's group into its stored `definitions`.
pub(in crate::creatures) fn keep(world: &mut World) {
    let patrols: Vec<(Entity, Patrol)> = world.query_filtered::<(Entity, &Patrol), With<Stored>>().iter(world).map(|(entity, patrol)| (entity, *patrol)).collect();
    for (entity, patrol) in patrols {
        let group = GROUPS[usize::from(patrol == Patrol::Follower)];
        entities::restore(world, entity, |compound, _| sapling_world::regrouped_actor(compound, &GROUPS, &[group]).ok());
    }
}

/// A stored pillager whose `definitions` hold a patrol's group rejoins a patrol.
pub(in crate::creatures) fn adopt(world: &mut World, entity: Entity, compound: &[u8]) {
    let groups = sapling_world::actor_groups(compound);
    match [0, 1].map(|at| groups.iter().any(|group| group == GROUPS[at])) {
        [true, _] => _ = world.entity_mut(entity).insert(NEW_CAPTAIN),
        [_, true] => _ = world.entity_mut(entity).insert(Patrol::Follower),
        _ => {}
    }
}

/// The end of the leg from `at` toward `goal`: the goal itself within a leg.
fn leg_toward(at: [f32; 3], goal: [f32; 3]) -> [f32; 3] {
    let off = flat(at, goal);
    if off <= LEG {
        return goal;
    }
    let along = |axis: usize| (at[axis] + (goal[axis] - at[axis]) / off * LEG).trunc() + 0.5;
    [along(0), at[1], along(2)]
}

/// Where a walk from `at` to `to` heads now: cornerwise while off on both axes, then straight.
fn cornerwise(at: [f32; 3], to: [f32; 3]) -> [f32; 3] {
    let (dx, dz) = (to[0] - at[0], to[2] - at[2]);
    if dx.abs() <= 0.5 || dz.abs() <= 0.5 {
        return to;
    }
    let both = dx.abs().min(dz.abs());
    [at[0] + dx.signum() * both, to[1], at[2] + dz.signum() * both]
}

/// Before a patrolling pillager's move, when it hunts nobody; false for one in no patrol, and for
/// a follower already by its captain or without one.
pub(in crate::creatures) fn steer(world: &mut World, entity: Entity, layer: Entity) -> bool {
    let (Some(patrol), Some(body)) = (world.get::<Patrol>(entity).copied(), world.get::<Body>(entity).copied()) else { return false };
    match patrol {
        Patrol::Captain { goal, leg } => {
            let goal = goal.filter(|goal| flat(*goal, body.at) > ARRIVED).unwrap_or_else(|| {
                let dice = &mut world.get_resource_or_insert_with(Dice::default);
                let [dx, dz] = [dice.fraction(), dice.fraction()].map(|roll| (roll * 2.0 - 1.0) * WANDERS);
                [body.at[0] + dx, body.at[1], body.at[2] + dz]
            });
            let leg = leg.filter(|leg| flat(*leg, body.at) > 1.0 && flat(*leg, goal) <= flat(body.at, goal)).unwrap_or_else(|| leg_toward(body.at, goal));
            world.entity_mut(entity).insert(Patrol::Captain { goal: Some(goal), leg: Some(leg) });
            hunting::approach(world, entity, layer, &body, cornerwise(body.at, leg), CAPTAIN_SPEED, 0.0);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_captain_walks_a_leg_cornerwise_then_straight() {
        assert_eq!(leg_toward([0.5, 64.0, 0.5], [100.5, 64.0, 0.5]), [32.5, 64.0, 0.5]);
        assert_eq!(leg_toward([0.5, 64.0, 0.5], [10.5, 64.0, 5.5]), [10.5, 64.0, 5.5]);
        assert_eq!(cornerwise([0.5, 64.0, 0.5], [10.5, 64.0, -4.5]), [5.5, 64.0, -4.5]);
        assert_eq!(cornerwise([5.5, 64.0, -4.5], [10.5, 64.0, -4.5]), [10.5, 64.0, -4.5]);
    }
}
