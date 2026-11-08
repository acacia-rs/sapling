//! A frog in water making for land (`behavior.move_to_land`: range 30, height 8, goal_radius 2;
//! `movement.amphibious`, `underwater_movement` 0.15; its pack has no `random_swim`). BDS's in
//! bds-f14efish.txt walked the pool floor at up to 0.11 a tick (0.98 x 0.15² = 0.022 a tick in
//! water's 0.8), its push coming up to strength over ten ticks, to below the rim, then rose up the
//! wall at up to 0.14 a tick and out onto it, 16 s from the middle of a pool 13 across. Sapling
//! heads straight for the nearest land and climbs once within [`UNDER`] of it across.
// TODO: BDS's way is an amphibious path, cell by cell; sapling's goes straight across, then up.

use bevy_ecs::prelude::*;
use sapling_world::World as Level;

use crate::body::Body;
use crate::components::{Layer, Tick};
use crate::creatures::{harm, legs};

const WATER: [&str; 2] = ["minecraft:water", "minecraft:flowing_water"];
const ACROSS: i32 = 30;
const UP: i32 = 8;
const PUSH: f32 = 0.022;
const PACE: f32 = 0.1;
/// Its push up the wall, fitted to the 0.14 a tick BDS's rose at.
const CLIMB: f32 = 0.048;
/// It climbs once this near its land across, while under the land's top.
const UNDER: f32 = 1.5;
const LOOKS_EVERY: u64 = 20;
const GIVES_UP: u64 = 400;

#[derive(Default, Clone, Copy)]
pub(super) struct Landward {
    to: Option<[f32; 3]>,
    since: u64,
    pace: f32,
}

/// Whether a frog may stand on `cell`'s top: a solid block, not water, with two cells of air over it.
fn is_land(level: &Level, [x, y, z]: [i32; 3]) -> bool {
    let state = |y: i32| level.registry().get(level.block(x, y, z));
    level.in_height(y)
        && state(y).is_some_and(|state| state.is_solid())
        && !harm::holds(level, [x, y, z], &WATER)
        && state(y + 1).is_some_and(|state| state.is_air())
        && state(y + 2).is_some_and(|state| state.is_air())
}

/// The middle of the top of the nearest land within reach of `at`.
fn nearest_land(level: &Level, at: [f32; 3]) -> Option<[f32; 3]> {
    let [x, y, z] = at.map(|c| c.floor() as i32);
    let far = |[cx, cy, cz]: &[i32; 3]| (cx - x).pow(2) + (cy - y).pow(2) + (cz - z).pow(2);
    let cells = (-ACROSS..=ACROSS).flat_map(|dx| (-UP..=UP).flat_map(move |dy| (-ACROSS..=ACROSS).map(move |dz| [x + dx, y + dy, z + dz])));
    cells.filter(|cell| is_land(level, *cell)).min_by_key(far).map(|[x, y, z]| [x as f32 + 0.5, y as f32 + 1.0, z as f32 + 0.5])
}

/// A tick of a frog in water: it heads for land, looked for every second while it has none.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, body: &Body, kept: &mut Landward) {
    let now = world.resource::<Tick>().0;
    if kept.to.is_none() || now >= kept.since + GIVES_UP {
        kept.to = None;
        if !now.is_multiple_of(LOOKS_EVERY) {
            return;
        }
        (kept.to, kept.since, kept.pace) = (world.get::<Layer>(layer).and_then(|layer| nearest_land(&layer.world, body.at)), now, 0.0);
    }
    let Some(land) = kept.to else { return };
    kept.pace = (kept.pace + PACE).min(1.0);
    let (dx, dz) = (land[0] - body.at[0], land[2] - body.at[2]);
    let far = dx.hypot(dz).max(1e-4);
    let up = if far < UNDER && body.at[1] < land[1] { CLIMB } else { 0.0 };
    let push = PUSH * kept.pace;
    if let Some(mut moving) = world.get_mut::<Body>(entity) {
        moving.fly([dx / far * push, up * kept.pace, dz / far * push]);
    }
    legs::face(world, entity, body, [land[0], land[2]]);
}
