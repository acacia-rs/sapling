//! Mobs with nothing to do stroll: now and then one walks to a place nearby; one a player hit
//! runs from place to place for a while (docs/research/mob-attacks.md, "Strolling", "Fleeing").
// How often: docs/research/mob-attacks.md "Strolling". TODO: a mob goes straight there and gives up at
// an edge or when it is too long on the way, where BDS's follows a path; none follows food. One the sun burns does not stroll out of the shade it stands in, as BDS's zombie
// under a roof did not (research/captures/bds-sun.txt), but one in the sun looks for no shade.

use bevy_ecs::prelude::*;

use super::legs;
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{Layer, Tick};

/// A tick in this many starts a stroll, to the first of this many places tried that will do.
const EVERY: f32 = 120.0;
const TRIES: usize = 10;
/// A stroll goes up to this far along each axis, ends this near its goal, and is given up after
/// this many ticks.
const FAR: f32 = 10.0;
const ARRIVED: f32 = 0.5;
const PATIENCE: u64 = 200;
/// A hit mob flees this many ticks, to places up to this far along each axis.
const FLEES: u64 = 70;
const FLEES_FAR: f32 = 5.0;
/// It looks for floor this far ahead of the middle of its box.
const AHEAD: f32 = 0.8;

/// Where a mob strolls to and the tick it gives up in, and the tick it flees until.
#[derive(Component, Default)]
pub(super) struct Stroll(Option<([f32; 2], u64)>, u64);

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

/// Whether a mob could stand in the cell of `[x, z]` at its feet's height: floor under it, nothing in it.
fn is_floor(world: &World, layer: Entity, body: &Body, [x, z]: [f32; 2]) -> bool {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return false };
    let [x, y, z] = [x, body.at[1], z].map(|c| c.floor() as i32);
    let holds = |y: i32| level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| !state.boxes.is_empty());
    level.is_loaded(sapling_world::ChunkPos::of_block(x, z)) && holds(y - 1) && !holds(y)
}

/// Before the move of a mob with nothing else to do: it starts a stroll now and then and walks
/// on one it has, at the speed it `strolls` at; one that `panics` and was hit runs at that.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, strolls: Option<f32>, panics: Option<f32>, shuns_sun: bool) {
    let now = world.resource::<Tick>().0;
    let (Some(body), Some((goal, until))) = (world.get::<Body>(entity).copied(), world.get::<Stroll>(entity).map(|stroll| (stroll.0, stroll.1))) else { return };
    let flees = panics.filter(|_| now < until);
    let (Some(speed), far) = (flees.or(strolls), if flees.is_some() { FLEES_FAR } else { FAR }) else { return };
    let goal = goal.or_else(|| {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        if !body.on_ground || (flees.is_none() && dice.fraction() * EVERY >= 1.0) {
            return None;
        }
        let tries: Vec<[f32; 2]> = (0..TRIES).map(|_| [body.at[0] + (dice.fraction() * 2.0 - 1.0) * far, body.at[2] + (dice.fraction() * 2.0 - 1.0) * far]).collect();
        let head = |[x, z]: [f32; 2]| [x, body.at[1] + body.size[1], z].map(|c| c.floor() as i32);
        let shaded = shuns_sun && !super::harm::sun_on(world, layer, head([body.at[0], body.at[2]]));
        let to = tries.into_iter().find(|to| !(shaded && super::harm::sun_on(world, layer, head(*to))) && is_floor(world, layer, &body, *to))?;
        Some((to, now + PATIENCE))
    });
    let ahead = goal.map(|(to, _)| {
        let (dx, dz) = (to[0] - body.at[0], to[1] - body.at[2]);
        let far = dx.hypot(dz).max(1e-4);
        (far, [body.at[0] + dx / far * AHEAD.min(far), body.at[2] + dz / far * AHEAD.min(far)])
    });
    let goes = goal.zip(ahead).filter(|((_, until), (far, ahead))| now < *until && *far > ARRIVED && (!body.on_ground || is_floor(world, layer, &body, *ahead)));
    match goes {
        Some(((to, _), _)) => legs::walk_turning(world, entity, layer, &body, to, speed, 0.0, legs::STROLL_TURN),
        None => legs::rest(world, entity),
    }
    if let Some(mut stroll) = world.get_mut::<Stroll>(entity) {
        stroll.0 = goes.map(|(goal, _)| goal);
    }
}
