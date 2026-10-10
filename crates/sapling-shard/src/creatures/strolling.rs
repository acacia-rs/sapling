//! Mobs with nothing to do stroll: now and then one walks to a place nearby
//! (docs/research/mob-attacks.md, "Strolling").
// TODO: how often and how far are fitted by eye to one zombie and one cow; a mob goes straight
// there and gives up at an edge or when it is too long on the way; none looks around, flees when
// hit, or follows food.

use bevy_ecs::prelude::*;

use super::legs;
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{Layer, Tick};

/// A tick in this many starts a stroll.
const EVERY: f32 = 120.0;
/// A stroll goes up to this far along each axis, ends this near its goal, and is given up after
/// this many ticks.
const FAR: f32 = 10.0;
const ARRIVED: f32 = 0.5;
const PATIENCE: u64 = 200;
/// It looks for floor this far ahead of the middle of its box.
const AHEAD: f32 = 0.8;

/// Where a mob strolls to, and the tick it gives up in.
#[derive(Component, Default)]
pub(super) struct Stroll(Option<([f32; 2], u64)>);

/// Whether a mob could stand in the cell of `[x, z]` at its feet's height: floor under it, nothing in it.
fn is_floor(world: &World, layer: Entity, body: &Body, [x, z]: [f32; 2]) -> bool {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return false };
    let [x, y, z] = [x, body.at[1], z].map(|c| c.floor() as i32);
    let holds = |y: i32| level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| !state.boxes.is_empty());
    level.is_loaded(sapling_world::ChunkPos::of_block(x, z)) && holds(y - 1) && !holds(y)
}

/// Before the move of a mob with nothing else to do: it starts a stroll now and then and walks
/// on one it has, at `speed`.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, speed: f32) {
    let now = world.resource::<Tick>().0;
    let (Some(body), Some(goal)) = (world.get::<Body>(entity).copied(), world.get::<Stroll>(entity).map(|stroll| stroll.0)) else { return };
    let goal = goal.or_else(|| {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        if !body.on_ground || dice.fraction() * EVERY >= 1.0 {
            return None;
        }
        let to = [body.at[0] + (dice.fraction() * 2.0 - 1.0) * FAR, body.at[2] + (dice.fraction() * 2.0 - 1.0) * FAR];
        is_floor(world, layer, &body, to).then_some((to, now + PATIENCE))
    });
    let ahead = goal.map(|(to, _)| {
        let (dx, dz) = (to[0] - body.at[0], to[1] - body.at[2]);
        let far = dx.hypot(dz).max(1e-4);
        (far, [body.at[0] + dx / far * AHEAD.min(far), body.at[2] + dz / far * AHEAD.min(far)])
    });
    let goes = goal.zip(ahead).filter(|((_, until), (far, ahead))| now < *until && *far > ARRIVED && (!body.on_ground || is_floor(world, layer, &body, *ahead)));
    match goes {
        Some(((to, _), _)) => legs::walk(world, entity, layer, &body, to, speed, 0.0),
        None => legs::rest(world, entity),
    }
    if let Some(mut stroll) = world.get_mut::<Stroll>(entity) {
        stroll.0 = goes.map(|(goal, _)| goal);
    }
}
