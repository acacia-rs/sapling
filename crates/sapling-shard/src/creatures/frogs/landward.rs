//! A frog in water making for land (`behavior.move_to_land`: range 30, height 8, goal_radius 2;
//! `movement.amphibious`, `underwater_movement` 0.15; its pack has no `random_swim`). BDS's in
//! bds-f14efish.txt walked the pool floor at up to 0.11 a tick (0.98 x 0.15² = 0.022 a tick in
//! water's 0.8), its push coming up to strength over ten ticks, to below the rim, then rose up the
//! wall at up to 0.14 a tick and out onto it, 16 s from the middle of a pool 13 across. Sapling
//! heads for land by an amphibious way (wading.rs), node by node: pushed across towards
//! the next node, and up or down by `AmphibiousMoveControl`'s rule (swimming.rs `amphibian_rise`);
//! within [`UNDER`] of the land across, under its top, it climbs. The land is BDS's
//! `BaseMoveToBlockGoal::findTargetBlock` pick (Linux 0x48c8b20, `search_count` 80): of 80 spots drawn at
//! random within reach, the nearest whose cell under it is land (`MoveToLandGoal` 0x5278840).

use bevy_ecs::prelude::*;
use sapling_world::World as Level;

use super::wading;
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{Layer, Tick};
use crate::creatures::{harm, legs, swimming};

const WATER: [&str; 2] = ["minecraft:water", "minecraft:flowing_water"];
const ACROSS: i32 = 30;
const UP: i32 = 8;
const TRIES: usize = 80;
const PUSH: f32 = 0.022;
const SPEED: f32 = 0.15;
const PACE: f32 = 0.1;
/// Its push up the wall, fitted to the 0.14 a tick BDS's rose at.
const CLIMB: f32 = 0.048;
/// It climbs once this near its land across, while under the land's top.
const UNDER: f32 = 1.5;
/// A node is passed once the frog is this near its middle across, and not under it by more.
const PASSED: f32 = 0.5;
const LOOKS_EVERY: u64 = 20;
const GIVES_UP: u64 = 400;

#[derive(Default, Clone)]
pub(super) struct Landward {
    to: Option<[f32; 3]>,
    way: Vec<[i32; 3]>,
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

/// [`TRIES`] spots at random within reach, as offsets from the frog's cell (`RandomPos::getPos` with one try each).
fn draw(dice: &mut Dice) -> Vec<[i32; 3]> {
    let mut off = |reach: i32| dice.roll(2 * reach as u32 + 1) as i32 - reach;
    (0..TRIES).map(|_| [off(ACROSS), off(UP), off(ACROSS)]).collect()
}

/// The nearest land of the cells under the `drawn` spots about `at`, as the cell over it.
fn nearest_land(level: &Level, at: [f32; 3], drawn: &[[i32; 3]]) -> Option<[i32; 3]> {
    let [x, y, z] = at.map(|c| c.floor() as i32);
    let far = |[cx, cy, cz]: &[i32; 3]| (cx - x).pow(2) + (cy - y).pow(2) + (cz - z).pow(2);
    let cells = drawn.iter().map(|[dx, dy, dz]| [x + dx, y + dy - 1, z + dz]);
    cells.filter(|cell| is_land(level, *cell)).min_by_key(far).map(|[x, y, z]| [x, y + 1, z])
}

/// The land, as the middle of its top, and the way there for a frog at `at`.
fn plan(level: &Level, at: [f32; 3], drawn: &[[i32; 3]]) -> Option<([f32; 3], Vec<[i32; 3]>)> {
    let land = nearest_land(level, at, drawn)?;
    let way = wading::route(level, at.map(|c| c.floor() as i32), land)?;
    Some(([land[0] as f32 + 0.5, land[1] as f32, land[2] as f32 + 0.5], way))
}

/// A tick of a frog in water: it heads for land, looked for every second while it has none.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, body: &Body, kept: &mut Landward) {
    let now = world.resource::<Tick>().0;
    if kept.to.is_none() || now >= kept.since + GIVES_UP {
        kept.to = None;
        if !now.is_multiple_of(LOOKS_EVERY) {
            return;
        }
        let drawn = draw(&mut world.get_resource_or_insert_with(Dice::default));
        let (to, way) = world.get::<Layer>(layer).and_then(|layer| plan(&layer.world, body.at, &drawn)).unzip();
        (kept.to, kept.way, kept.since, kept.pace) = (to, way.unwrap_or_default(), now, 0.0);
    }
    let Some(land) = kept.to else { return };
    while kept.way.len() > 1 && passed(body, kept.way[0]) {
        kept.way.remove(0);
    }
    let node = kept.way.first().map_or(land, |[x, y, z]| [*x as f32 + 0.5, *y as f32, *z as f32 + 0.5]);
    kept.pace = (kept.pace + PACE).min(1.0);
    let (dx, dz) = (node[0] - body.at[0], node[2] - body.at[2]);
    let far = dx.hypot(dz).max(1e-4);
    let climbs = (land[0] - body.at[0]).hypot(land[2] - body.at[2]) < UNDER && body.at[1] < land[1];
    let up = if climbs { CLIMB * kept.pace } else { swimming::amphibian_rise(SPEED, [dx, node[1] - body.at[1], dz], body.size[1]) };
    let push = PUSH * kept.pace;
    if let Some(mut moving) = world.get_mut::<Body>(entity) {
        moving.fly([dx / far * push, up, dz / far * push]);
    }
    legs::face(world, entity, body, [node[0], node[2]]);
}

/// Whether a frog at `body` is past the node `cell`: near its middle across, and not under it by more.
fn passed(body: &Body, [x, y, z]: [i32; 3]) -> bool {
    let across = (x as f32 + 0.5 - body.at[0]).hypot(z as f32 + 0.5 - body.at[2]);
    across < PASSED && y as f32 - body.at[1] < PASSED
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_land_is_the_nearest_drawn_spot_over_land_not_the_nearest_land() {
        let mut level = Level::void();
        let stone = level.block_id("minecraft:stone").unwrap();
        level.set_block(2, 64, 0, stone);
        level.set_block(6, 64, 0, stone);
        let at = [0.5, 65.0, 0.5];
        assert_eq!(nearest_land(&level, at, &[[6, 0, 0], [1, 3, 0]]), Some([6, 65, 0]), "the near stone was not drawn");
        assert_eq!(nearest_land(&level, at, &[[6, 0, 0], [2, 0, 0]]), Some([2, 65, 0]));
        assert_eq!(nearest_land(&level, at, &[[3, 0, 0], [2, 1, 0]]), None, "a spot counts the cell under it only");
    }
}
