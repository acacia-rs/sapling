//! A dolphin, as BDS's swam (research/captures/bds-dolphin-findings.md): in water it heads for a
//! spot of water up to 10 off across, or, once hit, its prey, which it bites as other hunters do
//! (`hunting::strike`); how it turns and is pushed: swim.rs. Short of breath it swims up and
//! breaches, and now and then breaches at the surface: breath.rs. Its anger: anger.rs; out of
//! water: ashore.rs; with no prey it may swim with a player: company.rs.
// TODO: it does not flock (`minecraft:flocking`); BDS's six in a pool 13 across kept 2.4 to 3.8 from
// their middle as it wandered (research/captures/bds-ab1flock.txt; docs/research/mob-kinds.md).

mod anger;
mod ashore;
mod breath;
mod company;
mod swim;

pub(super) use anger::{anger, bit};
pub(super) use ashore::dried;

use acacia_proto::types::MetadataFlags1;
use bevy_ecs::prelude::*;

use crate::blocks::Dice;
use crate::body::{Body, Liquid};
use crate::components::{Layer, NetId, Tick};
use crate::sync::Told;
use crate::entities;
use super::harm;

const KIND: &str = "minecraft:dolphin";
const WATER: &[&str] = &["minecraft:water", "minecraft:flowing_water"];
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
    /// The tick it came to rest on the ground, and the first it was out of water.
    landed: Option<u64>,
    ashore: Option<u64>,
    /// The ticks its `ANGRY` shows from and to, and whether it is shown.
    angry: Option<[u64; 2]>,
    shown: bool,
    /// Whether it swims up for breath; its breach under way, and the tick the last began.
    rising: bool,
    breach: Option<breath::Breach>,
    breached: Option<u64>,
    company: Option<company::Company>,
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

/// The spot it swims about to: the one it has, or a new one where that is reached or given up.
fn roam(world: &mut World, entity: Entity, layer: Entity, body: &Body) -> Option<[f32; 3]> {
    let now = world.resource::<Tick>().0;
    let (kept, since) = world.get::<Dolphin>(entity).map_or((None, 0), |dolphin| (dolphin.spot, dolphin.since));
    let near = kept.is_some_and(|spot| (0..3).map(|axis| (spot[axis] - body.at[axis]).powi(2)).sum::<f32>().sqrt() < REACHED);
    if let Some(spot) = kept.filter(|_| !near && now < since + GIVES_UP) {
        return Some(spot);
    }
    let found = spot(world, layer, body.at);
    if let Some(mut dolphin) = world.get_mut::<Dolphin>(entity) {
        (dolphin.spot, dolphin.since) = (found, now);
    }
    found
}

/// Shows or clears the dolphin's `flag`, where its viewers know it otherwise.
fn show_flag(world: &mut World, dolphin: Entity, layer: Entity, body: &Body, flag: MetadataFlags1, on: bool) {
    let known = world.get::<Told>(dolphin).and_then(Told::mob_flags).is_some_and(|flags| (flags.0 & flag.0 != 0) == on);
    let Some(id) = world.get::<NetId>(dolphin).map(|id| id.0).filter(|_| !known) else { return };
    let flags = entities::mob_flags(world, dolphin, KIND, flag, on);
    entities::tell_viewers(world, dolphin, layer, body.at.map(|c| c.floor() as i32), vec![crate::sync::flags_packet(id, flags)]);
}

/// Before its move, with its prey at `prey`: its anger shows or ends; in water it breaches,
/// swims up for breath, at its prey or about; out of it it flops.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, body: &Body, prey: Option<[f32; 3]>) -> bool {
    anger::join(world, entity, layer, body);
    anger::show(world, entity, layer, body);
    if breath::breaching(world, entity, layer, body) {
        return true;
    }
    if body.liquid() != Some(Liquid::Water) {
        ashore::flop(world, entity, body);
        return true;
    }
    if let Some(mut dolphin) = world.get_mut::<Dolphin>(entity) {
        dolphin.landed = None;
    }
    if breath::swim_up(world, entity, layer, body) || breath::leap(world, entity, layer, body) {
        return true;
    }
    match prey {
        Some(feet) => swim::chase(world, entity, body, [feet[0], feet[1] + MIDDLE, feet[2]]),
        None if company::swim_with(world, entity, body) => {}
        None => {
            if let Some(to) = roam(world, entity, layer, body) {
                swim::roam(world, entity, body, to);
            }
        }
    }
    swim::bob(world, entity, layer, body);
    true
}
