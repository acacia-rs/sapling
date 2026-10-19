//! A dolphin, as BDS's swam (research/captures/bds-dolphin-findings.md): in water it is pushed
//! 0.022 a tick (`Physics::SWIMMER`) towards a spot of water up to 10 off across, or, once hit, at
//! its prey, which it bites as other hunters do (`hunting::strike`).
// TODO: it does not run out of breath, breach, bob under the surface, turn 10 degrees a tick at
// most, flock or swim with players; out of water it neither flops nor dries out; its anger is not
// passed to dolphins near, not shown (`ANGRY`), and does not end.

use bevy_ecs::prelude::*;

use crate::blocks::Dice;
use crate::body::{Body, Liquid};
use crate::components::{Layer, Tick};
use super::harm;

const KIND: &str = "minecraft:dolphin";
const WATER: &[&str] = &["minecraft:water", "minecraft:flowing_water"];
const PUSH: f32 = 0.022;
/// A spot is up to this far off across and this far up or down; it is reached this near, or
/// given up after this many ticks, and looked for this many times.
const ROAMS: i32 = 10;
const DIVES: i32 = 3;
const REACHED: f32 = 1.0;
const GIVES_UP: u64 = 100;
const TRIES: usize = 10;
/// It swims at this much of its prey's height.
const MIDDLE: f32 = 0.9;

#[derive(Component, Default)]
pub(super) struct Dolphin {
    spot: Option<[f32; 3]>,
    since: u64,
}

pub(super) fn arm(world: &mut World, entity: Entity, kind: &str) {
    if kind == KIND {
        world.entity_mut(entity).insert(Dolphin::default());
    }
}

/// A spot of water near `at` in `layer`, if one is found.
fn spot(world: &mut World, layer: Entity, at: [f32; 3]) -> Option<[f32; 3]> {
    let cells: Vec<[i32; 3]> = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        let mut off = |reach: i32| dice.roll(2 * reach as u32 + 1) as i32 - reach;
        (0..TRIES).map(|_| [at[0].floor() as i32 + off(ROAMS), at[1].floor() as i32 + off(DIVES), at[2].floor() as i32 + off(ROAMS)]).collect()
    };
    let level = &world.get::<Layer>(layer)?.world;
    cells.into_iter().find(|cell| harm::holds(level, *cell, WATER)).map(|[x, y, z]| [x as f32 + 0.5, y as f32, z as f32 + 0.5])
}

/// Before its move, with its prey at `prey`: in water it swims at it, or about.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, body: &Body, prey: Option<[f32; 3]>) -> bool {
    if body.liquid() != Some(Liquid::Water) {
        return false;
    }
    let now = world.resource::<Tick>().0;
    let (kept, since) = world.get::<Dolphin>(entity).map_or((None, 0), |dolphin| (dolphin.spot, dolphin.since));
    let to = match prey {
        Some(feet) => [feet[0], feet[1] + MIDDLE, feet[2]],
        None => {
            let near = kept.is_some_and(|spot| (0..3).map(|axis| (spot[axis] - body.at[axis]).powi(2)).sum::<f32>().sqrt() < REACHED);
            match kept.filter(|_| !near && now < since + GIVES_UP) {
                Some(spot) => spot,
                None => {
                    let found = spot(world, layer, body.at);
                    if let Some(mut dolphin) = world.get_mut::<Dolphin>(entity) {
                        (dolphin.spot, dolphin.since) = (found, now);
                    }
                    let Some(found) = found else { return true };
                    found
                }
            }
        }
    };
    let towards: [f32; 3] = std::array::from_fn(|axis| to[axis] - body.at[axis]);
    let long = towards.iter().map(|c| c * c).sum::<f32>().sqrt();
    if long > 1e-4 {
        if let Some(mut moving) = world.get_mut::<Body>(entity) {
            moving.fly(towards.map(|c| c / long * PUSH));
        }
    }
    true
}
