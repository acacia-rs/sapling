//! Mobs with nothing to do stroll: now and then one walks to a place nearby; one a player hit
//! runs from place to place for a while (docs/research/mob-attacks.md, "Strolling", "Fleeing").
// How often: docs/research/mob-attacks.md "Strolling"; one tempted by food follows it instead (husbandry/breeding.rs `tempted`). One the sun burns does not stroll out of the shade it stands in, as BDS's zombie
// under a roof did not (research/captures/bds-sun.txt), but one in the sun looks for no shade.

use bevy_ecs::prelude::*;

use super::{legs, paths};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{Layer, Tick};

/// A tick in this many starts a stroll, to the first of this many places tried that will do.
const EVERY: f32 = 120.0;
const TRIES: usize = 10;
/// A stroll's place is drawn up to this far along each axis and this far up or down (`random_stroll`'s
/// `xz_dist`, `y_dist`); it takes the next cell of its way this near the middle of the last (half its
/// width if more: `PathNavigation::updatePath`, Linux 0xbd70f60), and is given up after this many ticks.
const FAR: (i32, i32) = (10, 7);
const ARRIVED: f32 = 0.6;
const PATIENCE: u64 = 200;
/// Within a block of its way's last middle its speed is the squared distance there, at least half:
/// BDS's MoveControl::calculateMoveSpeed 1430b8050 and shouldSlowSpeed 1430b90d0 (bds-strolls.txt
/// walks end 0.086, 0.076, 0.062, ... 0.022 a tick).
const LAST_SLOWEST: f32 = 0.5;
/// A hit mob flees this many ticks, to places drawn this far across and up (`PanicGoal::canUse`, Linux 0xbdc7260).
const FLEES: u64 = 70;
const FLEES_FAR: (i32, i32) = (5, 4);

type Cell = [i32; 3];

/// The cells left of a stroll's way and the tick it gives up in, the tick it flees until, and
/// whom it flees.
#[derive(Component, Default)]
pub(super) struct Stroll(Option<(Vec<Cell>, u64)>, u64, Option<Entity>);

/// A hit mob drops what it was going to and flees, away from `by` if a mob or player hurt it.
pub(super) fn startle(world: &mut World, entity: Entity, by: Option<Entity>) {
    let until = world.resource::<Tick>().0 + FLEES;
    if let Some(mut stroll) = world.get_mut::<Stroll>(entity) {
        (stroll.0, stroll.1, stroll.2) = (None, until, by.filter(|by| *by != entity));
    }
    // BDS's cow came up to its run from nothing after the throw.
    legs::rest(world, entity);
}

/// Whether the mob has somewhere to walk to: a stroll or a flight under way.
pub(crate) fn is_under_way(world: &World, entity: Entity) -> bool {
    let now = world.resource::<Tick>().0;
    world.get::<Stroll>(entity).is_some_and(|stroll| stroll.0.is_some() || now < stroll.1)
}

/// Whether the mob still flees a hit.
pub(super) fn is_fleeing(world: &World, entity: Entity) -> bool {
    world.get::<Stroll>(entity).is_some_and(|stroll| world.resource::<Tick>().0 < stroll.1)
}

/// The share of its speed a walker keeps on its way's last block, ending at `to` ([`LAST_SLOWEST`]).
pub(super) fn last_block_share(body: &Body, to: [f32; 2]) -> f32 {
    let near = (to[0] - body.at[0]).powi(2) + (to[1] - body.at[2]).powi(2);
    if near < 1.0 { near.max(LAST_SLOWEST) } else { 1.0 }
}

/// The way over whole cells toward a place drawn at random, as BDS's walks went from cell middle
/// to cell middle (research/captures/bds-strolls.txt): to the cell nearest it where it cannot be
/// reached, as BDS's zombies on a 3×3 top six up strolled about the top (bds-am2push.txt). Away from
/// `shunned`, a try across toward it is turned about: `RandomPos::getPosAvoid` (Linux 0xbd0b530 →
/// 0xbd0a310 negates the offsets whose dot with the way from it is below 0).
fn way_to(world: &mut World, layer: Entity, body: &Body, (far, up): (i32, i32), shuns_sun: bool, shunned: Option<[f32; 3]>) -> Option<Vec<Cell>> {
    let from = body.at.map(|c| c.floor() as i32);
    let away = shunned.map(|from| [body.at[0] - from[0], body.at[2] - from[2]]);
    let tries: Vec<Cell> = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        (0..TRIES)
            .map(|_| {
                let (mut dx, dy, mut dz) = (dice.roll(2 * far as u32 + 1) as i32 - far, dice.roll(2 * up as u32 + 1) as i32 - up, dice.roll(2 * far as u32 + 1) as i32 - far);
                if away.is_some_and(|[ax, az]| dx as f32 * ax + dz as f32 * az < 0.0) {
                    (dx, dz) = (-dx, -dz);
                }
                [from[0] + dx, from[1] + dy, from[2] + dz]
            })
            .collect()
    };
    let head = |[x, _, z]: Cell, feet: f32| [x, (feet + body.size[1]).floor() as i32, z];
    let shaded = shuns_sun && !super::harm::sun_on(world, layer, head(from, body.at[1]));
    let level = &world.get::<Layer>(layer)?.world;
    let to = tries.into_iter().filter_map(|cell| floor_of(level, cell, from[1] - up, from[1] + up).map(|y| [cell[0], y, cell[2]])).find(|to| !(shaded && super::harm::sun_on(world, layer, head(*to, to[1] as f32))))?;
    paths::route_toward(level, from, to).filter(|way| !way.is_empty())
}

/// The height a place drawn at `cell` is moved to, as BDS's `RandomPos` (Linux 0xbd0a310) moves it:
/// onto the solid cell it is in, else down its column to the first floor, else up, within `lo..=hi`.
fn floor_of(level: &sapling_world::World, [x, y, z]: Cell, lo: i32, hi: i32) -> Option<i32> {
    let solid = |y: i32| paths::holds(level, [x, y, z]);
    (lo..=y).rev().find(|y| solid(*y) && !solid(y + 1)).map(|y| y + 1).or_else(|| (y + 1..=hi).find(|y| !solid(*y) && solid(y - 1)))
}

/// Before the move of a mob with nothing else to do: it starts a stroll now and then and walks
/// its way on one it has, at the speed it `strolls` at; one that `panics` and was hit runs at that.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, strolls: Option<f32>, panics: Option<f32>, shuns_sun: bool) {
    let now = world.resource::<Tick>().0;
    let (Some(body), Some((stroll, until, by))) = (world.get::<Body>(entity).copied(), world.get_mut::<Stroll>(entity).map(|mut stroll| (stroll.0.take(), stroll.1, stroll.2))) else { return };
    let flees = panics.filter(|_| now < until);
    let (Some(speed), far) = (flees.or(strolls), if flees.is_some() { FLEES_FAR } else { FAR }) else { return };
    let shunned = by.filter(|_| flees.is_some()).and_then(|by| super::hunting::feet_of(world, by));
    let stroll = stroll.or_else(|| {
        let starts = body.on_ground && (flees.is_some() || world.get_resource_or_insert_with(Dice::default).fraction() * EVERY < 1.0);
        starts.then(|| way_to(world, layer, &body, far, shuns_sun, shunned)).flatten().map(|way| (way, now + PATIENCE))
    });
    let stroll = stroll.filter(|(_, until)| now < *until).map(|(way, until)| (walk_way(world, entity, layer, &body, way, speed), until));
    if stroll.is_none() {
        legs::rest(world, entity);
    }
    if let Some(mut kept) = world.get_mut::<Stroll>(entity) {
        kept.0 = stroll.filter(|(way, _)| !way.is_empty());
    }
}

/// The mob drops the stroll it was on, for something else to steer it.
pub(super) fn drop_stroll(world: &mut World, entity: Entity) {
    if let Some(mut stroll) = world.get_mut::<Stroll>(entity) {
        stroll.0 = None;
    }
}

/// Walks the mob along `way` at `speed`, slowing on its last block, or stands it still at its end:
/// what is left of the way, the cells it has reached dropped.
pub(super) fn walk_way(world: &mut World, entity: Entity, layer: Entity, body: &Body, mut way: Vec<Cell>, speed: f32) -> Vec<Cell> {
    let middle = |[x, _, z]: Cell| [x as f32 + 0.5, z as f32 + 0.5];
    let arrived = (body.size[0] * 0.5).max(ARRIVED);
    while way.first().is_some_and(|cell| {
        let [x, z] = middle(*cell);
        (x - body.at[0]).hypot(z - body.at[2]) < arrived
    }) {
        way.remove(0);
    }
    match way.first() {
        Some(cell) => {
            let to = middle(*cell);
            let slowed = if way.len() == 1 { last_block_share(body, to) } else { 1.0 };
            legs::walk(world, entity, layer, body, to, speed * slowed, 0.0);
        }
        None => legs::rest(world, entity),
    }
    way
}
