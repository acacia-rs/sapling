//! Kinds that swim about (`swims=` and `roams=` in mob-traits.tsv: fish, turtles, frogs,
//! axolotls), as BDS's swam in the `swimmers` and `amphibians` drills
//! (research/captures/bds-wander-findings.md): in water each tick it turns its body towards a spot
//! of water up to `roams` off and is pushed `0.98 x speed^2` along its yaw, and up or down
//! towards the spot; water keeps 0.8 of its speed (`Physics::SWIMMER`; a turtle's 0.9). It looks
//! for another spot once within a block of it or after 5 seconds. Out of water a turtle or an
//! axolotl walks to water within 16 blocks.
// TODO: the push up or down is fitted at its full strength only (a fish's 1.25 of its push
// across); nearer the spot's height it is taken to fall off over the last block. A fish out of
// water neither flops nor dies; none flees a player; a frog does not look for land.

use bevy_ecs::prelude::*;

use crate::blocks::Dice;
use crate::body::{Body, Liquid};
use crate::components::{Layer, Tick};
use crate::sync::Told;
use super::{harm, legs};

const WATER: &[&str] = &["minecraft:water", "minecraft:flowing_water"];
/// Its push a tick is this much of its speed squared.
const STROKE: f32 = 0.98;
/// A spot is reached this near, given up after this many ticks, and looked for this many times.
const REACHED: f32 = 1.0;
const GIVES_UP: u64 = 100;
const TRIES: usize = 10;
/// An amphibian's push comes up to strength by this much of it a tick.
const PACE: f32 = 0.1;
/// Out of water one that seeks water looks this far across and up or down for it, every so many ticks.
const SEEKS: i32 = 16;
const SEEKS_DEPTH: i32 = 5;
const SEEKS_EVERY: u64 = 20;

/// How a kind swims, by its pack's movement component: degrees it turns a tick (`max_turn`),
/// whether its push comes up to strength over ten ticks (`movement.amphibious`), how much harder
/// it is pushed up or down than across, and whether out of water it goes back.
#[derive(Clone, Copy)]
struct Stroke {
    turn: f32,
    ramps: bool,
    rise: f32,
    seeks_water: bool,
}

const FISH: Stroke = Stroke { turn: 30.0, ramps: false, rise: 1.25, seeks_water: false };

fn stroke(kind: &str) -> Stroke {
    match kind {
        "minecraft:turtle" => Stroke { turn: 5.0, ramps: true, rise: 1.0, seeks_water: true },
        "minecraft:axolotl" => Stroke { turn: 15.0, ramps: true, rise: 1.0, seeks_water: true },
        "minecraft:frog" => Stroke { turn: 30.0, ramps: true, rise: 1.0, seeks_water: false },
        _ => FISH,
    }
}

#[derive(Component, Default)]
pub(super) struct Swimmer {
    yaw: f32,
    pace: f32,
    spot: Option<[f32; 3]>,
    since: u64,
}

/// Makes a mob of a kind that swims about, and hunts nothing, a swimmer.
pub(super) fn arm(world: &mut World, entity: Entity, kind: &str) {
    let traits = sapling_data::mob_traits(kind);
    if traits.swims.is_some() && traits.hunts.is_none() {
        world.entity_mut(entity).insert(Swimmer::default());
    }
}

/// A cell holding water up to `reach` across and `depth` up or down from `at`, if one is found.
pub(super) fn water_near(world: &mut World, layer: Entity, at: [f32; 3], [reach, depth]: [i32; 2]) -> Option<[f32; 3]> {
    let cells: Vec<[i32; 3]> = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        let mut off = |reach: i32| dice.roll(2 * reach as u32 + 1) as i32 - reach;
        (0..TRIES).map(|_| [at[0].floor() as i32 + off(reach), at[1].floor() as i32 + off(depth), at[2].floor() as i32 + off(reach)]).collect()
    };
    let level = &world.get::<Layer>(layer)?.world;
    cells.into_iter().find(|cell| harm::holds(level, *cell, WATER)).map(|[x, y, z]| [x as f32 + 0.5, y as f32, z as f32 + 0.5])
}

/// The middle of the top of the nearest cell holding water within [`SEEKS`] across and
/// [`SEEKS_DEPTH`] up or down of `at`.
fn nearest_water(level: &sapling_world::World, at: [f32; 3]) -> Option<[f32; 3]> {
    let [x, y, z] = at.map(|c| c.floor() as i32);
    let cells = (-SEEKS..=SEEKS).flat_map(|dx| (-SEEKS_DEPTH..=SEEKS_DEPTH).flat_map(move |dy| (-SEEKS..=SEEKS).map(move |dz| [x + dx, y + dy, z + dz])));
    let far = |cell: &[i32; 3]| (cell[0] - x).pow(2) + (cell[1] - y).pow(2) + (cell[2] - z).pow(2);
    cells.filter(|cell| harm::holds(level, *cell, WATER)).min_by_key(far).map(|[x, y, z]| [x as f32 + 0.5, y as f32 + 1.0, z as f32 + 0.5])
}

/// The swimmer's spot: the one it has, or a new one where that is reached or given up.
fn spot(world: &mut World, entity: Entity, layer: Entity, body: &Body, roams: [u32; 2]) -> Option<[f32; 3]> {
    let now = world.resource::<Tick>().0;
    let swimmer = world.get::<Swimmer>(entity)?;
    let near = swimmer.spot.is_some_and(|spot| (0..3).map(|axis| (spot[axis] - body.at[axis]).powi(2)).sum::<f32>().sqrt() < REACHED);
    if let Some(spot) = swimmer.spot.filter(|_| !near && now < swimmer.since + GIVES_UP) {
        return Some(spot);
    }
    let found = water_near(world, layer, body.at, roams.map(|c| c as i32));
    let mut swimmer = world.get_mut::<Swimmer>(entity)?;
    (swimmer.spot, swimmer.since) = (found, now);
    found
}

/// Before its move: in water a swimmer swims to its spot; out of it one that seeks water walks
/// to some. False where it does neither, and strolls.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, kind: &str) -> bool {
    let (Some(body), Some((speed, roams))) = (world.get::<Body>(entity).copied(), sapling_data::mob_traits(kind).swims) else { return false };
    if world.get::<Swimmer>(entity).is_none() {
        return false;
    }
    let stroke = stroke(kind);
    if body.liquid() != Some(Liquid::Water) {
        if let Some(mut swimmer) = world.get_mut::<Swimmer>(entity) {
            swimmer.pace = 0.0;
        }
        return stroke.seeks_water && seek_water(world, entity, layer, kind, &body);
    }
    let Some(to) = spot(world, entity, layer, &body, roams) else { return true };
    let Some(mut swimmer) = world.get_mut::<Swimmer>(entity) else { return true };
    let (dx, dy, dz) = (to[0] - body.at[0], to[1] - body.at[1], to[2] - body.at[2]);
    let turn = ((-dx).atan2(dz).to_degrees() - swimmer.yaw + 180.0).rem_euclid(360.0) - 180.0;
    swimmer.yaw += turn.clamp(-stroke.turn, stroke.turn);
    swimmer.pace = if stroke.ramps { (swimmer.pace + PACE).min(1.0) } else { 1.0 };
    let (yaw, push) = (swimmer.yaw, STROKE * speed * speed * swimmer.pace);
    let facing = yaw.to_radians();
    if let Some(mut moving) = world.get_mut::<Body>(entity) {
        moving.fly([-facing.sin() * push, dy.clamp(-1.0, 1.0) * stroke.rise * push, facing.cos() * push]);
    }
    if let Some(mut told) = world.get_mut::<Told>(entity) {
        told.face(yaw, yaw);
    }
    if let Some(mut legs) = world.get_mut::<legs::Legs>(entity) {
        legs.yaw = yaw;
    }
    true
}

/// Out of water: it walks to water it found near, looked for every [`SEEKS_EVERY`] ticks.
fn seek_water(world: &mut World, entity: Entity, layer: Entity, kind: &str, body: &Body) -> bool {
    let now = world.resource::<Tick>().0;
    let Some(swimmer) = world.get::<Swimmer>(entity) else { return false };
    let kept = swimmer.spot.filter(|_| now < swimmer.since + GIVES_UP);
    let water = match kept {
        Some(water) => Some(water),
        None if now.is_multiple_of(SEEKS_EVERY) => {
            // Aimed a body's width past the water's edge: at the edge cell's middle a wide body still stands on the bank.
            let found = world.get::<Layer>(layer).and_then(|layer| nearest_water(&layer.world, body.at)).map(|[x, y, z]| {
                let (dx, dz) = (x - body.at[0], z - body.at[2]);
                let far = dx.hypot(dz).max(1e-4);
                [x + dx / far * body.size[0], y, z + dz / far * body.size[0]]
            });
            if let Some(mut swimmer) = world.get_mut::<Swimmer>(entity) {
                (swimmer.spot, swimmer.since) = (found, now);
            }
            found
        }
        None => None,
    };
    let (Some(water), Some(speed)) = (water, sapling_data::mob_traits(kind).strolls) else { return false };
    legs::walk(world, entity, layer, body, [water[0], water[2]], speed, 0.0);
    true
}
