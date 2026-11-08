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
/// A stroll goes up to this far along each axis, takes the next cell of its way this near the
/// middle of the last (half its width if more: `PathNavigation::updatePath`, Linux 0xbd70f60), and
/// is given up after this many ticks.
const FAR: f32 = 10.0;
const ARRIVED: f32 = 0.6;
const PATIENCE: u64 = 200;
/// Within a block of its way's last middle its speed is the squared distance there, at least half:
/// BDS's MoveControl::calculateMoveSpeed 1430b8050 and shouldSlowSpeed 1430b90d0 (bds-strolls.txt
/// walks end 0.086, 0.076, 0.062, ... 0.022 a tick).
const LAST_SLOWEST: f32 = 0.5;
/// A hit mob flees this many ticks, to places up to this far along each axis.
const FLEES: u64 = 70;
const FLEES_FAR: f32 = 5.0;

type Cell = [i32; 3];

/// The cells left of a stroll's way and the tick it gives up in, and the tick it flees until.
#[derive(Component, Default)]
pub(super) struct Stroll(Option<(Vec<Cell>, u64)>, u64);

/// A hit mob drops what it was going to and flees.
pub(super) fn startle(world: &mut World, entity: Entity) {
    let until = world.resource::<Tick>().0 + FLEES;
    if let Some(mut stroll) = world.get_mut::<Stroll>(entity) {
        (stroll.0, stroll.1) = (None, until);
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

/// The way over whole cells to a place tried at random, as BDS's walks went from cell middle to
/// cell middle (research/captures/bds-strolls.txt).
fn way_to(world: &mut World, layer: Entity, body: &Body, far: f32, shuns_sun: bool) -> Option<Vec<Cell>> {
    let tries: Vec<[f32; 2]> = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        (0..TRIES).map(|_| [body.at[0] + (dice.fraction() * 2.0 - 1.0) * far, body.at[2] + (dice.fraction() * 2.0 - 1.0) * far]).collect()
    };
    let head = |[x, z]: [f32; 2]| [x, body.at[1] + body.size[1], z].map(|c| c.floor() as i32);
    let shaded = shuns_sun && !super::harm::sun_on(world, layer, head([body.at[0], body.at[2]]));
    let from = body.at.map(|c| c.floor() as i32);
    let level = &world.get::<Layer>(layer)?.world;
    let suits = |to: [f32; 2]| !(shaded && super::harm::sun_on(world, layer, head(to)));
    tries.into_iter().filter(|to| suits(*to)).find_map(|[x, z]| {
        let to = [x.floor() as i32, from[1], z.floor() as i32];
        paths::is_standable(level, to).then(|| paths::route(level, from, to)).flatten().filter(|way| !way.is_empty())
    })
}

/// Before the move of a mob with nothing else to do: it starts a stroll now and then and walks
/// its way on one it has, at the speed it `strolls` at; one that `panics` and was hit runs at that.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, strolls: Option<f32>, panics: Option<f32>, shuns_sun: bool) {
    let now = world.resource::<Tick>().0;
    let (Some(body), Some((stroll, until))) = (world.get::<Body>(entity).copied(), world.get_mut::<Stroll>(entity).map(|mut stroll| (stroll.0.take(), stroll.1))) else { return };
    let flees = panics.filter(|_| now < until);
    let (Some(speed), far) = (flees.or(strolls), if flees.is_some() { FLEES_FAR } else { FAR }) else { return };
    let stroll = stroll.or_else(|| {
        let starts = body.on_ground && (flees.is_some() || world.get_resource_or_insert_with(Dice::default).fraction() * EVERY < 1.0);
        starts.then(|| way_to(world, layer, &body, far, shuns_sun)).flatten().map(|way| (way, now + PATIENCE))
    });
    let middle = |[x, _, z]: Cell| [x as f32 + 0.5, z as f32 + 0.5];
    let arrived = (body.size[0] * 0.5).max(ARRIVED);
    let stroll = stroll.filter(|(_, until)| now < *until).map(|(mut way, until)| {
        while way.first().is_some_and(|cell| {
            let [x, z] = middle(*cell);
            (x - body.at[0]).hypot(z - body.at[2]) < arrived
        }) {
            way.remove(0);
        }
        (way, until)
    });
    let stroll = stroll.filter(|(way, _)| !way.is_empty());
    match &stroll {
        Some((way, _)) => {
            let to = middle(way[0]);
            let slowed = if way.len() == 1 { last_block_share(&body, to) } else { 1.0 };
            legs::walk(world, entity, layer, &body, to, speed * slowed, 0.0);
        }
        None => legs::rest(world, entity),
    }
    if let Some(mut kept) = world.get_mut::<Stroll>(entity) {
        kept.0 = stroll;
    }
}
