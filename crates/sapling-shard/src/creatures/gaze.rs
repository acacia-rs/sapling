//! Where a mob left to itself looks, and how its body comes round after its head, as BDS's slowed
//! cows did (docs/research/mob-damage.md "Where a mob left to itself looks"; research/captures/
//! bds-gaze.txt, bds-glance.txt). `look_at_player` turns the head 10 degrees a tick towards a player
//! within its distance and pitches straight at the player's eyes; `random_look_around` puts the head
//! at once on a random side. Either keeps the head within 75 degrees of the body, which holds still
//! until the head has stayed put 10 ticks and then closes on it (BDS's body control, 0x142cc5810).
//! With no look the head comes back to the body at 10 degrees a tick; a stroll's turn ends a look.

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
/// How far the head may be from the body.
const HEAD_LEASH: f32 = 75.0;
/// A walking body comes round to where it walks this much a tick.
const BODY_TURN: f32 = 25.0;
/// The head is settled while it stays within this of where it settled; the body starts to close
/// on it after this many settled ticks, and is on it this many later.
const SETTLED: f32 = 15.0;
const SETTLES: u32 = 10;
/// A tick's move across, squared, past which the body walks.
const WALKS: f32 = 2.5e-7;
/// A mob's eyes are this share of its height up it, fitted to the cows' pitch.
const EYES: f32 = 0.95;
/// `random_look_around`: its chance a try and its look's ticks, fitted to bds-glance.txt's gaps.
const GLANCE_CHANCE: f32 = 0.02;
const GLANCE_TICKS: (u32, u32) = (40, 80);

#[derive(Clone, Copy)]
enum Goal {
    Idle,
    Player(Entity, u64),
    Side(f32, u64),
}

/// Its head's and its body's yaw, its goal, where the head settled and for how many ticks, and its
/// legs' yaw and feet last tick.
#[derive(Component, Clone, Copy)]
pub(super) struct Gaze {
    head: f32,
    body: f32,
    goal: Goal,
    settled: f32,
    still: u32,
    yaw: f32,
    at: [f32; 3],
    pitch: f32,
}

/// Where a mob's head faces, if it looks about.
pub(super) fn head_yaw(world: &World, entity: Entity) -> Option<f32> {
    world.get::<Gaze>(entity).map(|gaze| gaze.head)
}

fn apart(from: f32, to: f32) -> f32 {
    (to - from + 180.0).rem_euclid(360.0) - 180.0
}

/// From `from` towards `to` by at most `most` degrees.
fn towards(from: f32, to: f32, most: f32) -> f32 {
    from + apart(from, to).clamp(-most, most)
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
    // BDS's looks start half as often as their chance a tick says: as if tried every other tick (bds-glance.txt).
    if now % 2 == 1 {
        return kept.unwrap_or(Goal::Idle);
    }
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
        let side = roll(world) * 360.0 - 180.0;
        return Goal::Side(side, now + ticks(world, GLANCE_TICKS));
    }
    Goal::Idle
}

/// The entity's body is turned to `yaw` (a `/tp` with a turn): a mob's legs and gaze start over
/// from it, its head following a tick later (bds-g8rot.txt).
pub(super) fn turn(world: &mut World, entity: Entity, yaw: f32) {
    if let Some(mut legs) = world.get_mut::<Legs>(entity) {
        legs.yaw = yaw;
    }
    world.entity_mut(entity).remove::<Gaze>();
    if let Some(mut told) = world.get_mut::<Told>(entity) {
        told.turn_body(yaw);
    }
}

/// The mob `id`, new this tick, is pitched `pitch` in its announcement.
pub(super) fn pitch_new(world: &mut World, id: sapling_api::EntityId, pitch: f32) {
    let Some(entity) = crate::entities::find(world, id).filter(|_| pitch != 0.0) else { return };
    if let Some(mut told) = world.get_mut::<Told>(entity) {
        told.pitch(pitch);
    }
    super::shown::reshow(world, entity);
}

/// After its legs were steered: the mob of `kind`, left to itself, turns its head and pitches, and
/// its body comes round.
pub(super) fn tick(world: &mut World, entity: Entity, layer: Entity, kind: &str, body: &Body) {
    let traits = sapling_data::mob_traits(kind);
    let Some(yaw) = world.get::<Legs>(entity).map(|legs| legs.yaw).filter(|_| traits.looks.is_some() || traits.glances) else { return };
    let Some(&gaze) = world.get::<Gaze>(entity) else {
        // The announced pitch first; a summoned one comes back level from the first tick (bds-h1summon.txt).
        let announced = world.get::<Told>(entity).map_or(0.0, |told| told.facing()[1]);
        let pitch = towards(announced, 0.0, HEAD_TURN);
        world.entity_mut(entity).insert(Gaze { head: yaw, body: yaw, goal: Goal::Idle, settled: yaw, still: 0, yaw, at: body.at, pitch });
        if let Some(mut told) = world.get_mut::<Told>(entity) {
            told.pitch(announced);
            told.pitch(pitch);
        }
        return;
    };
    let eyes = [body.at[0], body.at[1] + body.size[1] * EYES, body.at[2]];
    let kept = if yaw != gaze.yaw { Goal::Idle } else { gaze.goal };
    let goal = goal(world, layer, eyes, kept, traits.looks, traits.glances);
    let (head, pitch) = match goal {
        Goal::Player(player, _) => {
            let at = world.get::<Position>(player).map_or(eyes, |position| [position.feet[0], position.feet[1] + sapling_data::EYE_HEIGHT, position.feet[2]]);
            let [dx, dy, dz] = [at[0] - eyes[0], at[1] - eyes[1], at[2] - eyes[2]];
            let wanted = towards(gaze.body, (-dx).atan2(dz).to_degrees(), HEAD_LEASH);
            (towards(gaze.head, wanted, HEAD_TURN), -dy.atan2(dx.hypot(dz)).to_degrees())
        }
        Goal::Side(side, _) => (towards(gaze.body, side, HEAD_LEASH), towards(gaze.pitch, 0.0, HEAD_TURN)),
        Goal::Idle => (towards(gaze.head, gaze.body, HEAD_TURN), towards(gaze.pitch, 0.0, HEAD_TURN)),
    };
    let walked = (body.at[0] - gaze.at[0]).powi(2) + (body.at[2] - gaze.at[2]).powi(2) > WALKS;
    let (settled, still, turned) = if walked {
        (head, 0, towards(gaze.body, yaw, BODY_TURN))
    } else if (head - gaze.settled).abs() > SETTLED {
        (head, 0, gaze.body)
    } else {
        let still = gaze.still + 1;
        let leash = HEAD_LEASH * (1.0 - still.saturating_sub(SETTLES) as f32 / SETTLES as f32).max(0.0);
        (gaze.settled, still, towards(head, gaze.body, leash))
    };
    world.entity_mut(entity).insert(Gaze { head, body: turned, goal, settled, still, yaw, at: body.at, pitch });
    if let Some(mut told) = world.get_mut::<Told>(entity) {
        told.face(yaw, head);
        told.pitch(pitch);
    }
}
