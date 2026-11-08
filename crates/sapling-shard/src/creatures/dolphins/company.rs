//! A dolphin swimming with a player (`behavior.swim_with_entity`), as Linux BDS's
//! `SwimWithEntityGoal` (vtable 0xeeb43d8) runs it. canUse (0x533fc00): in water, a draw under
//! `success_rate` 0.1, then a player at random within 20 that is `SWIMMING` or rides something
//! moving. canContinueToUse (0x53402e0): while its player is neither, a draw under `chance_to_stop`
//! 0.0333 ends it; it goes on while its way is unfinished or its player is over 5 off. Every
//! `state_check_interval` (10 ticks; tick 0x5340650) it aims at the cell under its player's, moved
//! 3 to the side of the player's heading it is on, and on along the heading: 2 while catching up,
//! till within 2 of that; 6 while keeping pace, till over 12 from the player. It swims there as it
//! swims about (swim.rs: BDS's dolphins swam no faster for a goal's speed).

use bevy_ecs::prelude::*;

use super::{Dolphin, swim};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{Position, in_world};
use crate::movement::Motion;
use crate::riding::Riding;

const SUCCESS: u32 = 10;
const STOPS: f32 = 0.0333;
const CHECKS_EVERY: u32 = 10;
const SEARCH: f32 = 20.0;
const STOP: f32 = 5.0;
const CATCH_UP: f32 = 12.0;
const MATCHED: f32 = 2.0;
const SIDE: f32 = 3.0;
const AHEAD: [f32; 2] = [2.0, 6.0];
const STILL: f32 = 1e-4;
/// A way is done this near its cell.
const REACHED: f32 = 1.0;

#[derive(Clone, Copy)]
pub(super) struct Company {
    player: Entity,
    keeping_pace: bool,
    wait: u32,
    to: Option<[f32; 3]>,
}

/// The way a player goes, if it swims or rides something moving: its own step, or its mount's across.
fn going(world: &World, player: Entity) -> Option<[f32; 3]> {
    if let Some(riding) = world.get::<Riding>(player) {
        let [x, _, z] = world.get::<Body>(riding.vehicle)?.motion;
        return (x * x + z * z > STILL).then_some([x, 0.0, z]);
    }
    let motion = world.get::<Motion>(player)?;
    motion.pose.swimming.then_some(motion.delta)
}

fn unit(v: [f32; 3]) -> [f32; 3] {
    let long = v.iter().map(|c| c * c).sum::<f32>().sqrt();
    if long < STILL { [0.0; 3] } else { v.map(|c| c / long) }
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|axis| (a[axis] - b[axis]).powi(2)).sum::<f32>().sqrt()
}

/// A player to go with, at a draw: one within reach that swims or rides.
fn pick(world: &mut World, body: &Body) -> Option<Entity> {
    if world.get_resource_or_insert_with(Dice::default).roll(SUCCESS) != 0 {
        return None;
    }
    let near: Vec<Entity> = in_world(world)
        .into_iter()
        .filter(|player| world.get::<Position>(*player).is_some_and(|at| (0..3).all(|axis| (at.feet[axis] - body.at[axis]).abs() <= SEARCH)))
        .filter(|player| going(world, *player).is_some())
        .collect();
    (!near.is_empty()).then(|| near[world.get_resource_or_insert_with(Dice::default).roll(near.len() as u32) as usize])
}

/// The cell it aims at: the one under its player's, 3 to the side it is on, and `ahead` on.
fn aim(feet: [f32; 3], way: [f32; 3], at: [f32; 3], ahead: f32) -> [f32; 3] {
    let heading = unit(way);
    let side = unit([-heading[2], 0.0, heading[0]]);
    let off = unit(std::array::from_fn(|axis| at[axis] - feet[axis]));
    let dot: f32 = (0..3).map(|axis| off[axis] * side[axis]).sum();
    let lean = if dot.abs() <= f32::EPSILON { 0.2 } else { dot };
    let cell: [i32; 3] = std::array::from_fn(|axis| (feet[axis].floor() + 0.5 + side[axis] * lean * SIDE).floor() as i32 + (heading[axis] * ahead).floor() as i32);
    [cell[0] as f32 + 0.5, (cell[1] - 1) as f32, cell[2] as f32 + 0.5]
}

/// With no prey: it swims with a player, starting at a draw. Whether it did this tick.
pub(super) fn swim_with(world: &mut World, entity: Entity, body: &Body) -> bool {
    let kept = world.get::<Dolphin>(entity).and_then(|dolphin| dolphin.company);
    let mut company = match kept {
        Some(company) => company,
        None => match pick(world, body) {
            Some(player) => Company { player, keeping_pace: false, wait: 0, to: None },
            None => return false,
        },
    };
    let Some(feet) = world.get::<Position>(company.player).map(|at| at.feet) else { return part(world, entity) };
    let way = going(world, company.player);
    if way.is_none() && world.get_resource_or_insert_with(Dice::default).fraction() < STOPS {
        return part(world, entity);
    }
    let unfinished = company.to.is_some_and(|to| distance(to, body.at) > REACHED);
    if kept.is_some() && !unfinished && distance(feet, body.at) <= STOP {
        return part(world, entity);
    }
    if company.wait <= 1 {
        company.wait = CHECKS_EVERY;
        let to = aim(feet, way.unwrap_or([0.0; 3]), body.at, AHEAD[usize::from(company.keeping_pace)]);
        let flips = if company.keeping_pace { distance(body.at, feet) > CATCH_UP } else { distance(body.at, to) < MATCHED };
        if flips {
            (company.keeping_pace, company.wait) = (!company.keeping_pace, 0);
        }
        company.to = Some(to);
    } else {
        company.wait -= 1;
    }
    if let Some(mut dolphin) = world.get_mut::<Dolphin>(entity) {
        dolphin.company = Some(company);
    }
    if let Some(to) = company.to {
        swim::chase(world, entity, body, to);
    }
    true
}

/// It leaves its player. False: it did not swim with one this tick.
fn part(world: &mut World, entity: Entity) -> bool {
    if let Some(mut dolphin) = world.get_mut::<Dolphin>(entity) {
        dolphin.company = None;
    }
    false
}
