//! Kinds that swim about (`swims=` and `roams=` in mob-traits.tsv: fish, turtles, frogs,
//! axolotls), as BDS's swam in the `swimmers` and `amphibians` drills
//! (research/captures/bds-wander-findings.md): in water each tick it turns its body towards a spot
//! of water up to `roams` off and is pushed `0.98 x speed^2` along its yaw, and up or down
//! towards the spot by [`rise`]; water keeps 0.8 of its speed (`Physics::SWIMMER`; a turtle's 0.9).
//! It looks for another spot once within a block of it or after 5 seconds. Out of water a turtle or
//! an axolotl walks to water within 16 blocks.
//! Fish hop out of water and flee players (ashore.rs); frogs make for land (frogs/landward.rs).
// BDS signs a fish's push up or down by its pitch, which sapling does not keep: it goes by the spot's height.

mod ashore;

pub(super) use ashore::{heard_flop, is_fish};

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

/// A fish's push up or down is its push across times this much of the height to its spot, at most [`FISH_RISE_MOST`].
const FISH_RISE: f32 = 5.0;
const FISH_RISE_MOST: f32 = 1.25;
/// An amphibian's push up or down is its speed times this, along the way to its spot.
const AMPHIBIAN_RISE: f32 = 0.1;

/// How a kind swims, by its pack's movement component: degrees it turns a tick (`max_turn`),
/// whether it is amphibious (`movement.amphibious`: its push comes up to strength over ten ticks,
/// and it rises by [`rise`]'s other rule), and whether out of water it goes back.
#[derive(Clone, Copy)]
struct Stroke {
    turn: f32,
    ramps: bool,
    seeks_water: bool,
}

const FISH: Stroke = Stroke { turn: 30.0, ramps: false, seeks_water: false };

fn stroke(kind: &str) -> Stroke {
    match kind {
        "minecraft:turtle" => Stroke { turn: 5.0, ramps: true, seeks_water: true },
        "minecraft:axolotl" => Stroke { turn: 15.0, ramps: true, seeks_water: true },
        "minecraft:frog" => Stroke { turn: 30.0, ramps: true, seeks_water: false },
        _ => FISH,
    }
}

/// A swimmer's push up or down towards a spot `off` from its feet. A fish's (`SwimMoveControl` tick, Linux
/// 0xbdf8d50) is its push across times 5 x the height off, at most 1.25 of it: full strength until the last
/// quarter block. An amphibian's: [`amphibian_rise`].
fn rise(stroke: Stroke, push: f32, speed: f32, off: [f32; 3], height: f32) -> f32 {
    if stroke.ramps { amphibian_rise(speed, off, height) } else { (off[1] * FISH_RISE).clamp(-FISH_RISE_MOST, FISH_RISE_MOST) * push }
}

/// An amphibian's push up or down towards a spot `off` from its feet (`AmphibiousMoveControl` tick, Linux
/// 0xbe27b10): its `speed` x 0.1 along the way there, the height off taken half its `height` further.
pub(in crate::creatures) fn amphibian_rise(speed: f32, [dx, dy, dz]: [f32; 3], height: f32) -> f32 {
    let dy = if dy > 0.0 { dy + height / 2.0 } else { dy - height / 2.0 };
    let far = (dx * dx + dy * dy + dz * dz).sqrt();
    if far > 0.0 { speed * AMPHIBIAN_RISE * dy / far } else { 0.0 }
}

#[derive(Component, Default)]
pub(super) struct Swimmer {
    yaw: f32,
    pace: f32,
    spot: Option<[f32; 3]>,
    since: u64,
    /// Whether its spot is one it flees a player to.
    fleeing: bool,
    /// Out of water, the tick it was first seen on the ground.
    landed: Option<u64>,
}

/// Makes a mob of a kind that swims about, and hunts nothing or swims about between its prey (a
/// guardian), a swimmer.
pub(super) fn arm(world: &mut World, entity: Entity, kind: &str) {
    let traits = sapling_data::mob_traits(kind);
    if traits.swims.is_some() && (traits.hunts.is_none() || super::guardians::is_guardian(kind)) {
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
            (swimmer.pace, swimmer.fleeing) = (0.0, false);
        }
        if is_fish(kind) {
            ashore::flop(world, entity, &body);
            return true;
        }
        return stroke.seeks_water && seek_water(world, entity, layer, kind, &body);
    }
    let haste = ashore::flee(world, entity, layer, kind, &body);
    let Some(to) = spot(world, entity, layer, &body, roams) else { return true };
    let Some(mut swimmer) = world.get_mut::<Swimmer>(entity) else { return true };
    swimmer.landed = None;
    let (dx, dy, dz) = (to[0] - body.at[0], to[1] - body.at[1], to[2] - body.at[2]);
    let turn = ((-dx).atan2(dz).to_degrees() - swimmer.yaw + 180.0).rem_euclid(360.0) - 180.0;
    swimmer.yaw += turn.clamp(-stroke.turn, stroke.turn);
    swimmer.pace = if stroke.ramps { (swimmer.pace + PACE).min(1.0) } else { 1.0 };
    let (yaw, push) = (swimmer.yaw, STROKE * speed * speed * swimmer.pace * haste);
    let facing = yaw.to_radians();
    let up = rise(stroke, push, speed, [dx, dy, dz], body.size[1]);
    if let Some(mut moving) = world.get_mut::<Body>(entity) {
        moving.fly([-facing.sin() * push, up, facing.cos() * push]);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fish_rises_at_full_strength_until_the_last_quarter_block() {
        let push = 0.0098;
        assert!((rise(FISH, push, 0.1, [3.0, 2.0, 0.0], 0.6) - 1.25 * push).abs() < 1e-7);
        assert!((rise(FISH, push, 0.1, [3.0, -0.25, 0.0], 0.6) + 1.25 * push).abs() < 1e-7);
        assert!((rise(FISH, push, 0.1, [3.0, 0.1, 0.0], 0.6) - 0.5 * push).abs() < 1e-7);
    }

    #[test]
    fn an_amphibian_rises_at_a_tenth_of_its_speed_along_its_way() {
        let frog = stroke("minecraft:frog");
        // 1.5 up and half its 0.5 height: (0, 1.75, 0) from feet, straight up.
        assert!((rise(frog, 0.022, 0.15, [0.0, 1.5, 0.0], 0.5) - 0.015).abs() < 1e-7);
        let way = rise(frog, 0.022, 0.15, [4.0, 0.0, 0.0], 0.5);
        assert!((way - 0.015 * -0.25 / 4.0f32.hypot(0.25)).abs() < 1e-7, "{way}");
    }
}
