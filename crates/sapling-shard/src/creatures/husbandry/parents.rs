//! A baby follows a grown one of its kind (`behavior.follow_parent`): BDS's `FollowParentGoal` (Linux
//! vtable 0xeee5ba8; canUse 0x57420e0, canContinueToUse 0x57425a0, tick 0x57427d0), Java's rule. A
//! baby takes the nearest grown one of its kind whose box is near its own ([`SEARCH`]), if that one is
//! 3 or more off, and follows it while it is 3 to 16 off: its way to that one's
//! cell is found again every 10 ticks, walked at its `movement` times the goal's `speed_multiplier`.

use bevy_ecs::prelude::*;

use crate::body::Body;
use crate::components::{InLayer, Layer, Tick};
use crate::creatures::{Dying, Health, genes, jockeys, paths, shown, strolling};
use crate::entities::Kind;

/// The baby's box grows by this across and up or down to find a grown one whose box it meets (canUse's
/// fetch, 0x574228f; gdb in bds-ai2kin: every take was 8.68 or less across, babies 9 to 11 off took
/// none until a stroll brought them in; bds-af2baby's piglet "9.72 off" had strolled in so).
const SEARCH: [f32; 2] = [8.0, 4.0];
/// It follows one from 3 off to 16 off, squared.
const NEAREST: f32 = 9.0;
const FARTHEST: f32 = 256.0;
const REROUTES: u64 = 10;

type Cell = [i32; 3];

/// The grown one a baby follows, the way left to it, and the tick its way is found again.
#[derive(Component)]
pub(super) struct Following {
    parent: Entity,
    way: Vec<Cell>,
    reroutes: u64,
}

fn squared(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|axis| (a[axis] - b[axis]).powi(2)).sum()
}

/// Where `mob` stands, if it is a grown one alive in `layer`.
fn grown(world: &World, mob: Entity, layer: Entity) -> Option<Body> {
    let alive = world.get::<Health>(mob).is_some_and(|health| health.left() > 0.0) && world.get::<Dying>(mob).is_none();
    let here = world.get::<InLayer>(mob).is_some_and(|in_layer| in_layer.0 == layer);
    world.get::<Body>(mob).copied().filter(|_| alive && here && !shown::is_baby(world, mob))
}

/// The nearest grown one of `kind` within the search box about `body`, if 3 or more off.
fn nearest(world: &mut World, entity: Entity, layer: Entity, kind: &str, body: &Body) -> Option<Entity> {
    let kin: Vec<Entity> = world.query::<(Entity, &Kind)>().iter(world).filter(|(other, of)| *other != entity && of.0 == kind).map(|(other, _)| other).collect();
    let within = |other: &Body| {
        let across = (body.size[0] + other.size[0]) * 0.5 + SEARCH[0];
        let below = other.at[1] + other.size[1] >= body.at[1] - SEARCH[1];
        (body.at[0] - other.at[0]).abs() < across && (body.at[2] - other.at[2]).abs() < across && below && other.at[1] <= body.at[1] + body.size[1] + SEARCH[1]
    };
    let found = kin.into_iter().filter_map(|other| grown(world, other, layer).filter(within).map(|at| (other, squared(body.at, at.at))));
    found.min_by(|a, b| a.1.total_cmp(&b.1)).filter(|(_, near)| *near >= NEAREST).map(|(other, _)| other)
}

/// Before the move of a baby: it follows its parent, or starts to. False for one that does not.
pub(super) fn follow(world: &mut World, entity: Entity, layer: Entity, kind: &str) -> bool {
    let traits = sapling_data::mob_traits(kind);
    let (Some(share), Some(body)) = (traits.follows_parent, world.get::<Body>(entity).copied()) else { return false };
    let follows = shown::is_baby(world, entity) && !strolling::is_fleeing(world, entity);
    let kept = world.get::<Following>(entity).map(|following| following.parent).filter(|_| follows);
    let kept = kept.filter(|&parent| grown(world, parent, layer).is_some_and(|at| (NEAREST..=FARTHEST).contains(&squared(body.at, at.at))));
    // A cow's stroll is as urgent as its following: one under way goes on (bds-af2baby.txt).
    let starts = follows && !(traits.strolls_on && strolling::is_under_way(world, entity));
    let Some(parent) = kept.or_else(|| starts.then(|| nearest(world, entity, layer, kind, &body)).flatten()) else {
        world.entity_mut(entity).remove::<Following>();
        return false;
    };
    let now = world.resource::<Tick>().0;
    if kept.is_none() {
        strolling::drop_stroll(world, entity);
        world.entity_mut(entity).insert(Following { parent, way: Vec::new(), reroutes: now });
    }
    let Following { way, reroutes, .. } = world.entity_mut(entity).take::<Following>().expect("inserted above");
    let way = if now >= reroutes { route(world, layer, &body, parent).unwrap_or_default() } else { way };
    let reroutes = if now >= reroutes { now + REROUTES } else { reroutes };
    let speed = jockeys::movement(world, entity, kind).unwrap_or(0.0) * share * genes::pace(world, entity);
    let way = strolling::walk_way(world, entity, layer, &body, way, speed);
    world.entity_mut(entity).insert(Following { parent, way, reroutes });
    true
}

fn route(world: &World, layer: Entity, body: &Body, parent: Entity) -> Option<Vec<Cell>> {
    let level = &world.get::<Layer>(layer)?.world;
    let to = world.get::<Body>(parent)?.at.map(|c| c.floor() as i32);
    paths::route(level, body.at.map(|c| c.floor() as i32), to)
}
