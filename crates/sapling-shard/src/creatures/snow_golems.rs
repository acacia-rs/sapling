//! Snow golems, as the pack's and BDS's in the `snowgolem` drill (research/captures/bds-snowgolem.txt).
//! One takes the nearest monster within 6 (`nearest_attackable_target`; `TargetEid` its unique id,
//! `mob.warning` a tick later) and walks at it for the 20 ticks `ranged_attack` takes to see it,
//! or while it is over 10 off; 21 ticks after taking it, and every 20 after, it throws a snowball
//! at it (BDS: 22, the golem knocked about). Each tick it leaves a layer of snow where it is cold
//! enough (`trail`, under 0.81), under the four corners a quarter block about where it stood, as
//! BDS laid them; it melts where it is hot (over 1.0) and in water or rain (`hurt_on_condition`).
// TODO: BDS's cause for the heat is `temperature`, which the API lacks (fire here); monsters do not
// fight back, nor does a hit golem take its hitter.

use bevy_ecs::prelude::*;

use super::arrows::{self, ARROW, Lands, Shot};
use super::{Creature, Dying, harm, hunting, legs};
use crate::body::{Body, Liquid};
use crate::components::{InLayer, Layer, NetId, Tick};
use crate::entities::{self, Kind};
use crate::late_sounds;

const GOLEM: &str = "minecraft:snow_golem";
/// Monsters it takes within this, looking every `LOOKS` ticks, and lets go beyond `FOLLOWS`.
const SIGHT: f32 = 6.0;
const LOOKS: u64 = 10;
const FOLLOWS: f32 = 16.0;
/// It throws within this, the first throw this long after taking its prey and then every `THROWS`.
const RANGE: f32 = 10.0;
const FIRST: u64 = 21;
const THROWS: u64 = 20;
/// Its movement 0.2 times `ranged_attack`'s `speed_multiplier`.
const SPEED: f32 = 0.25;
/// Snow lies under it below the first temperature; it melts above the second.
const SNOWS_UNDER: f32 = 0.81;
const MELTS_OVER: f32 = 1.0;
const CORNER: f32 = 0.25;
const SNOW: &str = "minecraft:snow_layer";
/// The pack's `minecraft:projectile`: from its eyes less 0.1 at 1.5 a tick.
pub(super) const SNOWBALL: Shot = Shot { kind: "minecraft:snowball", ahead: 0.0, held: 1.6, speed: 1.5, weight: 0.03, sticks: false, throws: false, sound: "shoot", lands: Lands::Snowball, ..ARROW };

#[derive(Component)]
pub(super) struct Golem {
    prey: Option<Entity>,
    since: u64,
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|axis| (a[axis] - b[axis]).powi(2)).sum::<f32>().sqrt()
}

/// Before its move: at its prey, if it has one; false to stroll.
pub(super) fn steer(world: &mut World, golem: Entity, layer: Entity, id: u64, kind: &str) -> bool {
    if kind != GOLEM {
        return false;
    }
    let Some(body) = world.get::<Body>(golem).copied() else { return false };
    let Some(prey) = pick(world, golem, layer, id, &body) else { return false };
    let Some(at) = world.get::<Body>(prey).map(|prey| prey.at) else { return false };
    let now = world.resource::<Tick>().0;
    let since = world.get::<Golem>(golem).map_or(now, |golem| golem.since);
    let far = distance(body.at, at);
    let near = if far > RANGE || now < since + THROWS { 0.0 } else { f32::INFINITY };
    legs::walk(world, golem, layer, &body, [at[0], at[2]], SPEED, near);
    let due = now >= since + FIRST && (now - since - FIRST).is_multiple_of(THROWS);
    if due && far <= RANGE {
        arrows::loose(world, layer, golem, kind, &body, at, None, SNOWBALL);
    }
    true
}

/// Keeps or picks the golem's prey, telling its viewers of a change.
fn pick(world: &mut World, golem: Entity, layer: Entity, id: u64, body: &Body) -> Option<Entity> {
    let now = world.resource::<Tick>().0;
    let had = world.get::<Golem>(golem).and_then(|golem| golem.prey);
    let kept = had.filter(|prey| super::is_alive(world, *prey) && world.get::<Body>(*prey).is_some_and(|prey| distance(prey.at, body.at) <= FOLLOWS));
    let prey = kept.or_else(|| (now + id).is_multiple_of(LOOKS).then(|| nearest(world, golem, layer, body)).flatten());
    if prey != had {
        world.entity_mut(golem).insert(Golem { prey, since: now });
        // The unique id its viewers were told of it, as hunting's.
        let target = prey.and_then(|prey| world.get::<NetId>(prey)).map_or(0, |id| id.0 as i64);
        let cell = body.at.map(|c| c.floor() as i32);
        entities::tell_viewers(world, golem, layer, cell, vec![hunting::targets(id, target)]);
        if prey.is_some() {
            late_sounds::play_later(world, 1, layer, cell, hunting::sound("mob.warning", id, GOLEM, body, hunting::HEAD));
        }
    }
    prey
}

fn nearest(world: &mut World, golem: Entity, layer: Entity, body: &Body) -> Option<Entity> {
    world
        .query_filtered::<(Entity, &InLayer, &Kind, &Body), (With<Creature>, Without<Dying>)>()
        .iter(world)
        .filter(|(mob, on, kind, at)| *mob != golem && on.0 == layer && distance(at.at, body.at) <= SIGHT && sapling_data::mob_families(&kind.0).contains(&"monster"))
        .min_by(|a, b| distance(a.3.at, body.at).total_cmp(&distance(b.3.at, body.at)))
        .map(|(mob, ..)| mob)
}

/// Before every golem's move: its trail and its melting.
pub(super) fn tick(world: &mut World) {
    let golems: Vec<(Entity, Entity, Body)> = world
        .query_filtered::<(Entity, &InLayer, &Kind, &Body), (With<Creature>, Without<Dying>)>()
        .iter(world)
        .filter(|(_, _, kind, _)| kind.0 == GOLEM)
        .map(|(golem, layer, _, body)| (golem, layer.0, *body))
        .collect();
    let weather = |world: &World, layer| crate::weather::of(world, layer);
    for (golem, layer, body) in golems {
        let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { continue };
        let [x, y, z] = body.at.map(|c| c.floor() as i32);
        let warmth = level.temperature(x, y, z);
        let rained = harm::is_rained_on(level, [x, y, z], weather(world, layer));
        if warmth.is_some_and(|t| t < SNOWS_UNDER) {
            leave_snow(world, layer, &body);
        }
        if body.liquid() == Some(Liquid::Water) || rained {
            harm::wound(world, golem, layer, 1.0, super::Cause::Drowning);
        } else if warmth.is_some_and(|t| t > MELTS_OVER) && super::afflictions::level(world, golem, sapling_api::Effect::FireResistance).is_none() {
            harm::wound(world, golem, layer, 1.0, super::Cause::Fire);
        }
    }
}

fn leave_snow(world: &mut World, layer: Entity, body: &Body) {
    for corner in 0..4 {
        let dx = if corner % 2 == 0 { -CORNER } else { CORNER };
        let dz = if corner < 2 { -CORNER } else { CORNER };
        let pos = [(body.at[0] + dx).floor() as i32, body.at[1].floor() as i32, (body.at[2] + dz).floor() as i32];
        let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
        let fits = level.in_height(pos[1] - 1) && level.is_loaded(sapling_world::ChunkPos::of_block(pos[0], pos[2])) && level.block(pos[0], pos[1], pos[2]) == level.registry().air_id() && crate::blocks::holds_snow(level, level.block(pos[0], pos[1] - 1, pos[2]));
        if fits {
            crate::blocks::put_block(world, layer, pos, &sapling_api::Block::new(SNOW));
        }
    }
}
