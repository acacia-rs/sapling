//! Where a mob left to itself looks, as BDS's slowed cows did (research/captures/bds-gaze.txt,
//! bds-tickcircle4.txt): `look_at_player` turns its head 10 degrees a tick towards a player within
//! its distance and pitches straight at the player's eyes, for its look time; then the pitch is 0
//! again and the head stays. `random_look_around` turns the head alone to a random side. With
//! neither, a head stays put until the body turns, then comes round to it at the same 10 degrees.

use bevy_ecs::prelude::*;
use sapling_data::Looks;

use super::legs::Legs;
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Mode, Position, Tick, in_world};
use sapling_api::GameMode;
use crate::sync::Told;

/// Degrees the head turns in a tick (7 of the wire's 256ths a tick).
const HEAD_TURN: f32 = 10.0;
/// A mob's eyes are this share of its height up it, fitted to the cows' pitch.
const EYES: f32 = 0.95;
/// `random_look_around`: its chance a tick, its look's ticks, and how far from the body it looks.
const GLANCE_CHANCE: f32 = 0.02;
const GLANCE_TICKS: (u32, u32) = (20, 40);
const GLANCE_SIDE: f32 = 90.0;

#[derive(Clone, Copy)]
enum Goal {
    Idle,
    Player(Entity, u64),
    Side(f32, u64),
}

/// Its head's yaw, its goal, its body's yaw last tick, and whether the head comes round to a
/// body that turned (a look's end leaves it where it was).
#[derive(Component, Clone, Copy)]
pub(super) struct Gaze {
    head: f32,
    goal: Goal,
    body_yaw: f32,
    follows: bool,
}

fn turned(from: f32, to: f32) -> f32 {
    from + ((to - from + 180.0).rem_euclid(360.0) - 180.0).clamp(-HEAD_TURN, HEAD_TURN)
}

fn roll(world: &mut World) -> f32 {
    world.get_resource_or_insert_with(Dice::default).fraction()
}

fn ticks(world: &mut World, (least, most): (u32, u32)) -> u64 {
    u64::from(least + world.get_resource_or_insert_with(Dice::default).roll(most - least + 1))
}

/// The player in `layer` nearest `at`, within `distance`, and its eyes; a spectator is not looked
/// at, as the Java game's look goal.
fn player_near(world: &World, layer: Entity, at: [f32; 3], distance: f32) -> Option<(Entity, [f32; 3])> {
    let eyes = |player: Entity| world.get::<Position>(player).map(|position| [position.feet[0], position.feet[1] + sapling_data::EYE_HEIGHT, position.feet[2]]);
    let far = |eyes: [f32; 3]| (eyes[0] - at[0]).hypot(eyes[2] - at[2]).hypot(eyes[1] - at[1]);
    let lookable = |player: &Entity| world.get::<InLayer>(*player).map(|layer| layer.0) == Some(layer) && world.get::<Mode>(*player).is_none_or(|mode| mode.0 != GameMode::Spectator);
    let near = in_world(world).into_iter().filter(lookable).filter_map(|player| Some((player, eyes(player)?)));
    near.filter(|(_, eyes)| far(*eyes) <= distance).min_by(|a, b| far(a.1).total_cmp(&far(b.1)))
}

/// The goal the mob keeps or takes this tick.
fn goal(world: &mut World, layer: Entity, eyes: [f32; 3], kept: Goal, looks: Option<Looks>, glances: bool) -> Goal {
    let now = world.resource::<Tick>().0;
    let kept = match kept {
        Goal::Player(player, until) if now < until && looks.is_some_and(|looks| player_near(world, layer, eyes, looks.distance).is_some_and(|(near, _)| near == player)) => return kept,
        Goal::Side(_, until) if now < until => Some(kept),
        _ => None,
    };
    if let Some(looks) = looks
        && roll(world) < looks.chance
        && let Some((player, _)) = player_near(world, layer, eyes, looks.distance)
    {
        return Goal::Player(player, now + ticks(world, looks.ticks));
    }
    if let Some(kept) = kept {
        return kept;
    }
    if glances && roll(world) < GLANCE_CHANCE {
        let side = (roll(world) * 2.0 - 1.0) * GLANCE_SIDE;
        return Goal::Side(side, now + ticks(world, GLANCE_TICKS));
    }
    Goal::Idle
}

/// After its legs were steered: the mob of `kind`, left to itself, turns its head and pitches.
pub(super) fn tick(world: &mut World, entity: Entity, layer: Entity, kind: &str, body: &Body) {
    let traits = sapling_data::mob_traits(kind);
    let Some(yaw) = world.get::<Legs>(entity).map(|legs| legs.yaw).filter(|_| traits.looks.is_some() || traits.glances) else { return };
    let Some(&Gaze { head: was, goal: kept, body_yaw, follows }) = world.get::<Gaze>(entity) else {
        world.entity_mut(entity).insert(Gaze { head: yaw, goal: Goal::Idle, body_yaw: yaw, follows: false });
        // The first pitch is the one viewers were told at its announcement.
        if let Some(mut told) = world.get_mut::<Told>(entity) {
            told.pitch(0.0);
        }
        return;
    };
    let eyes = [body.at[0], body.at[1] + body.size[1] * EYES, body.at[2]];
    let goal = goal(world, layer, eyes, kept, traits.looks, traits.glances);
    let (head, pitch) = match goal {
        Goal::Player(player, _) => {
            let at = world.get::<Position>(player).map_or(eyes, |position| [position.feet[0], position.feet[1] + sapling_data::EYE_HEIGHT, position.feet[2]]);
            let [dx, dy, dz] = [at[0] - eyes[0], at[1] - eyes[1], at[2] - eyes[2]];
            (turned(was, (-dx).atan2(dz).to_degrees()), -dy.atan2(dx.hypot(dz)).to_degrees())
        }
        Goal::Side(side, _) => (turned(was, yaw + side), 0.0),
        Goal::Idle if follows || yaw != body_yaw => (turned(was, yaw), 0.0),
        Goal::Idle => (was, 0.0),
    };
    let follows = matches!(goal, Goal::Idle) && apart(head, yaw);
    world.entity_mut(entity).insert(Gaze { head, goal, body_yaw: yaw, follows });
    if let Some(mut told) = world.get_mut::<Told>(entity) {
        // Its legs face the head with the body where they walked; an unmoved head is left told.
        if apart(head, yaw) || apart(head, was) {
            told.face(yaw, head);
        }
        told.pitch(pitch);
    }
}

fn apart(a: f32, b: f32) -> bool {
    ((a - b + 180.0).rem_euclid(360.0) - 180.0).abs() > 0.01
}
