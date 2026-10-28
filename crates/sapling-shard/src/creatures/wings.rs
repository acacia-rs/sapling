//! Bats, allays, bees (bees.rs steers them to flowers, hives and prey) and parrots flying about, as BDS's flew in the `flyers` drill
//! (research/captures/bds-wander-findings.md), on the ghast's way of flying (ghasts.rs): each tick
//! one is pushed along the line to its spot, after its kind's drag (`Physics::BAT`, `HOVER`,
//! `PARROT`). A bat is pushed 0.1 and picks a spot up to 10 off across, 7 up or down and 2 lower
//! every 2 to 7 ticks (its `float_wander`); an allay is pushed 0.0098, coming up to it over ten
//! ticks, to a spot up to 8 off and 1 lower, 1 to 4 over the ground (`random_hover`); a parrot
//! 0.0188 to one up to 15 off across. Either picks another within a block of its spot or after 5
//! seconds.
// TODO: a bat does not hang from a ceiling; a parrot neither sits, lands nor falls, bobs where
// it hovers, nor flies higher than its pack's 1 block up or down (BDS's rose 8 over the floor, which
// `HOVERS` copies); an allay neither follows a player nor takes items.

use bevy_ecs::prelude::*;

use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{Layer, Tick};
use crate::sync::Told;

const REACHED: f32 = 1.0;
const GIVES_UP: u64 = 100;
const TRIES: usize = 10;
/// The pace an allay's push comes up to strength by, a tick.
const PACE: f32 = 0.1;
/// How far down a spot's ground is looked for.
const GROUND: i32 = 16;

/// How a kind flies about: its push, whether that comes up to strength, how far off a spot is
/// across and up or down and how much lower, how high over the ground it must be, and how often
/// one is picked where not only once it is reached.
struct Wing {
    push: f32,
    ramps: bool,
    across: f32,
    up: f32,
    lower: f32,
    over: Option<[f32; 2]>,
    every: Option<[u32; 2]>,
}

const BAT: Wing = Wing { push: 0.1, ramps: false, across: 10.0, up: 7.0, lower: 2.0, over: None, every: Some([2, 7]) };
const ALLAY: Wing = Wing { push: 0.0098, ramps: true, across: 8.0, up: 8.0, lower: 1.0, over: Some([1.0, 4.0]), every: None };
/// A bee hovers as an allay does with its own speed, 0.98 x 0.15² (research/captures/bds-keephive.txt:
/// from rest 0.0063, 0.0121, 0.0193 ... up to 0.1627 a tick, as the allay's ramp gives).
const BEE: Wing = Wing { push: 0.02205, ..ALLAY };
/// A parrot's spot is kept this high over the ground (see the TODO).
const HOVERS: [f32; 2] = [2.0, 8.0];
const PARROT: Wing = Wing { push: 0.0188, ramps: false, across: 15.0, up: 1.0, lower: 0.0, over: Some(HOVERS), every: None };

fn wing(kind: &str) -> Option<&'static Wing> {
    match kind {
        "minecraft:bat" => Some(&BAT),
        "minecraft:allay" => Some(&ALLAY),
        super::bees::KIND => Some(&BEE),
        "minecraft:parrot" => Some(&PARROT),
        _ => None,
    }
}

#[derive(Component, Default)]
pub(super) struct Wings {
    spot: Option<[f32; 3]>,
    until: u64,
    pace: f32,
}

pub(super) fn arm(world: &mut World, entity: Entity, kind: &str) {
    if wing(kind).is_some() {
        world.entity_mut(entity).insert(Wings::default());
    }
}

/// A free spot about `at` by `wing`, if one is found in [`TRIES`].
fn pick(world: &mut World, layer: Entity, at: [f32; 3], wing: &Wing) -> Option<[f32; 3]> {
    let picks: Vec<[f32; 3]> = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        let mut off = |reach: f32| (dice.fraction() * 2.0 - 1.0) * reach;
        (0..TRIES).map(|_| [at[0] + off(wing.across), at[1] + off(wing.up) - wing.lower, at[2] + off(wing.across)]).collect()
    };
    let level = &world.get::<Layer>(layer)?.world;
    let solid = |[x, y, z]: [i32; 3]| level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| !state.boxes.is_empty());
    picks.into_iter().find_map(|mut spot| {
        if let Some([low, high]) = wing.over {
            let [x, y, z] = spot.map(|c| c.floor() as i32);
            let ground = (0..GROUND).map(|down| y - down).find(|y| solid([x, *y, z]))? + 1;
            spot[1] = spot[1].clamp(ground as f32 + low, ground as f32 + high);
        }
        let cell = spot.map(|c| c.floor() as i32);
        (level.is_loaded(sapling_world::ChunkPos::of_block(cell[0], cell[2])) && !solid(cell)).then_some(spot)
    })
}

/// Before its move: it flies on to its spot, picking another when due.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, kind: &str) -> bool {
    let (Some(wing), Some(body)) = (wing(kind), world.get::<Body>(entity).copied()) else { return false };
    let now = world.resource::<Tick>().0;
    let Some(wings) = world.get::<Wings>(entity) else { return false };
    let near = wings.spot.is_some_and(|spot| (0..3).map(|axis| (spot[axis] - body.at[axis]).powi(2)).sum::<f32>().sqrt() < REACHED);
    let spot = match wings.spot.filter(|_| !near && now < wings.until) {
        Some(spot) => Some(spot),
        None => {
            let picked = pick(world, layer, body.at, wing);
            let lasts = wing.every.map_or(GIVES_UP, |[least, most]| u64::from(least + world.get_resource_or_insert_with(Dice::default).roll(most - least + 1)));
            if let Some(mut wings) = world.get_mut::<Wings>(entity) {
                (wings.spot, wings.until) = (picked, now + lasts);
            }
            picked
        }
    };
    if let Some(spot) = spot {
        fly_to(world, entity, wing, &body, spot);
    }
    true
}

/// Pushes the flyer a tick along the line to `spot` by its kind's wing, and turns it that way.
fn fly_to(world: &mut World, entity: Entity, wing: &Wing, body: &Body, spot: [f32; 3]) {
    let towards: [f32; 3] = std::array::from_fn(|axis| spot[axis] - body.at[axis]);
    let long = towards.iter().map(|c| c * c).sum::<f32>().sqrt();
    let Some(mut wings) = world.get_mut::<Wings>(entity) else { return };
    wings.pace = if wing.ramps { (wings.pace + PACE).min(1.0) } else { 1.0 };
    let push = wing.push * wings.pace;
    if long > 1e-4 {
        if let Some(mut moving) = world.get_mut::<Body>(entity) {
            moving.fly(towards.map(|c| c / long * push));
        }
        let yaw = (-towards[0]).atan2(towards[2]).to_degrees();
        if let Some(mut told) = world.get_mut::<Told>(entity) {
            told.face(yaw, yaw);
        }
    }
}

/// Flies a flyer of `kind` a tick at `spot` (a bee to its flower, its hive or its prey), its
/// spot about given up; false for a kind that does not fly about.
pub(super) fn fly_at(world: &mut World, entity: Entity, kind: &str, spot: [f32; 3]) -> bool {
    let (Some(wing), Some(body)) = (wing(kind), world.get::<Body>(entity).copied()) else { return false };
    if let Some(mut wings) = world.get_mut::<Wings>(entity) {
        wings.spot = None;
    }
    fly_to(world, entity, wing, &body, spot);
    true
}
