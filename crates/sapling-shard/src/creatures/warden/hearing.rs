//! What disturbs a warden: vibrations it hears (`VibrationDetected` when one reaches it, at most
//! one every 2 seconds), players touching it (once a second) and players it sniffs out; each
//! angers it at the player behind it and sends it to look (research/captures/bds-warden-findings.md).

use acacia_proto::packets::{EntityEvent, EntityEventEventId};
use bevy_ecs::prelude::*;

use super::{HEAD, Stage, Warden, anger};
use crate::blocks::GameEvent;
use crate::body::Body;
use crate::components::{InLayer, Position, Tick, in_world};
use crate::creatures::hunting;
use crate::{entities, late_sounds, vitals};

/// It hears vibrations this far off, each reaching it a tick a block; after one it hears none for this long.
const HEARS: f32 = 16.0;
const DEAF_FOR: u64 = 40;
/// A player touching it, boxes this near, disturbs it once in this many ticks.
const TOUCH_GRACE: f32 = 0.2;
const TOUCHES: u64 = 20;
const PLAYER: [f32; 2] = [0.6, 1.8];

struct Coming {
    when: u64,
    warden: Entity,
    at: [f32; 3],
    source: Option<Entity>,
    projectile: bool,
}

#[derive(Resource, Default)]
pub(super) struct Heard {
    coming: Vec<Coming>,
}

/// `event` happened at `at`, by `source` where known: each warden of `layer` in range that is
/// listening starts to wait for it.
pub(crate) fn hear(world: &mut World, layer: Entity, at: [f32; 3], event: GameEvent, source: Option<Entity>) {
    if event.frequency() == 0 {
        return;
    }
    let now = world.resource::<Tick>().0;
    let all: Vec<Entity> = world.query_filtered::<Entity, With<Warden>>().iter(world).collect();
    if all.is_empty() && world.get_resource::<Heard>().is_none() {
        return;
    }
    let mut heard = world.get_resource_or_insert_with(Heard::default);
    heard.coming.retain(|coming| all.contains(&coming.warden));
    let waiting: Vec<Entity> = heard.coming.iter().map(|coming| coming.warden).collect();
    let near: Vec<(Entity, f32)> = world
        .query::<(Entity, &InLayer, &Body, &Warden)>()
        .iter(world)
        .filter(|(warden, in_layer, _, state)| in_layer.0 == layer && Some(*warden) != source && state.listens(now) && !waiting.contains(warden))
        .map(|(warden, _, body, _)| (warden, (0..3).map(|axis| (body.at[axis] - at[axis]).powi(2)).sum::<f32>().sqrt()))
        .filter(|(_, far)| *far <= HEARS)
        .collect();
    let projectile = matches!(event, GameEvent::ProjectileLand | GameEvent::ProjectileShoot);
    let mut heard = world.resource_mut::<Heard>();
    for (warden, far) in near {
        heard.coming.push(Coming { when: now + (far as u64).max(1), warden, at, source, projectile });
    }
}

/// The vibration on its way to the warden reached it: it is told, and angered at a player that made it.
pub(super) fn arrive(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body) {
    let now = world.resource::<Tick>().0;
    let Some(mut heard) = world.get_resource_mut::<Heard>() else { return };
    let Some(at) = heard.coming.iter().position(|coming| coming.warden == entity && coming.when <= now) else { return };
    let Coming { at, source, projectile, .. } = heard.coming.swap_remove(at);
    let cell = body.at.map(|c| c.floor() as i32);
    let told = EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::VibrationDetected, data: 0, fire_at_position: None };
    entities::tell_viewers(world, entity, layer, cell, vec![sapling_net::encode(&told)]);
    if let Some(mut warden) = world.get_mut::<Warden>(entity) {
        warden.deaf_until = now + DEAF_FOR;
    }
    let suspect = source.filter(|player| is_suspect(world, *player, layer));
    disturb(world, entity, layer, id, kind, body, suspect.map(|player| (player, if projectile { anger::PROJECTILE_ANNOYS } else { anger::ANNOYS })), at, 1);
}

/// A player it may be angry at: a mortal one of its layer.
pub(super) fn is_suspect(world: &World, player: Entity, layer: Entity) -> bool {
    world.get::<InLayer>(player).is_some_and(|in_layer| in_layer.0 == layer) && vitals::is_mortal(world, player)
}

/// Each player whose box touches the warden's disturbs it, once a second.
pub(super) fn touched(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body) {
    let now = world.resource::<Tick>().0;
    if world.get::<Warden>(entity).is_none_or(|warden| now < warden.touch_ready) {
        return;
    }
    let touches = |feet: [f32; 3]| {
        let across = (body.size[0] + PLAYER[0]) / 2.0 + TOUCH_GRACE;
        (feet[0] - body.at[0]).abs() < across && (feet[2] - body.at[2]).abs() < across && feet[1] < body.at[1] + body.size[1] && body.at[1] < feet[1] + PLAYER[1]
    };
    let Some((player, feet)) = in_world(world).into_iter().filter(|player| is_suspect(world, *player, layer)).filter_map(|player| Some((player, world.get::<Position>(player)?.feet))).find(|(_, feet)| touches(*feet)) else { return };
    world.get_mut::<Warden>(entity).expect("read above").touch_ready = now + TOUCHES;
    disturb(world, entity, layer, id, kind, body, Some((player, anger::ANNOYS)), feet, 0);
}

/// Something at `at` disturbed the warden, angering it by an amount at a player: it is heard
/// listening `after` ticks on (a tick after a vibration reached it, at once for a touch) and goes
/// to look where it is not yet angry.
#[allow(clippy::too_many_arguments)]
pub(super) fn disturb(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body, by: Option<(Entity, u32)>, at: [f32; 3], after: u64) {
    let now = world.resource::<Tick>().0;
    let Some(mut warden) = world.get_mut::<Warden>(entity) else { return };
    warden.calm_since = now;
    let sound = by.map(|(player, anger)| warden.anger.raise(player, anger));
    if warden.anger.level() < anger::ANGRY && matches!(warden.stage, Stage::Idle) {
        warden.disturbance = Some(at);
    }
    let (Some(sound), cell) = (sound, body.at.map(|c| c.floor() as i32)) else { return };
    let sound = hunting::sound(sound, id, kind, body, HEAD);
    match after {
        0 => entities::tell_viewers(world, entity, layer, cell, vec![sound]),
        _ => late_sounds::play_later(world, after, layer, cell, sound),
    }
}
