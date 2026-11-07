//! Where a hunter walks: straight at its prey where the floor lets it, else by a way round, and
//! where it stops short of it.

use bevy_ecs::prelude::*;

use super::super::{legs, paths};
use super::Hunter;
use crate::body::Body;
use crate::components::{Layer, Tick};

/// A way round is looked for again after this many ticks, and a cell of it is reached this near
/// its middle.
const REPLANS: u64 = 10;
const ARRIVED: f32 = 0.35;
/// Walking straight at its prey, the prey is its way's last node, reached this near (or half its
/// width): `PathNavigation`'s node radius, as a stroll's (strolling.rs). See [`stops_short`].
const NODE_REACHED: f32 = 0.6;

/// Where a hunter walks to this tick, and whether straight at its prey: so where the floor lets
/// it, else to the next cell of a way round, looked for anew every [`REPLANS`] ticks.
pub(super) fn heads(world: &mut World, entity: Entity, layer: Entity, body: &Body, feet: [f32; 3]) -> ([f32; 2], bool) {
    let now = world.resource::<Tick>().0;
    let straight = ([feet[0], feet[2]], true);
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return straight };
    if paths::is_clear(level, body.at, feet) {
        return straight;
    }
    let stale = world.get::<Hunter>(entity).is_none_or(|hunter| hunter.way.is_empty() || now >= hunter.planned + REPLANS);
    let found = stale.then(|| paths::route(level, body.at.map(|c| c.floor() as i32), feet.map(|c| c.floor() as i32)).unwrap_or_default());
    let Some(mut hunter) = world.get_mut::<Hunter>(entity) else { return straight };
    if let Some(way) = found {
        (hunter.way, hunter.planned) = (way, now);
    }
    let middle = |cell: &[i32; 3]| [cell[0] as f32 + 0.5, cell[2] as f32 + 0.5];
    while hunter.way.first().is_some_and(|cell| (middle(cell)[0] - body.at[0]).hypot(middle(cell)[1] - body.at[2]) < ARRIVED) {
        hunter.way.remove(0);
    }
    hunter.way.first().map_or(straight, |cell| (middle(cell), false))
}

/// Whether the hunter, walking `straight` at its prey at `to`, stops short of it this tick: its
/// last walk began within the node radius, so its path follower ended the path, a tick behind its
/// move (bds-ah3stop2.txt: the husk pushed from 0.5955 off, having been 0.6321 off, and stood
/// from 0.5626). Keeps this walk's start for the next.
pub(super) fn stops_short(world: &mut World, entity: Entity, body: &Body, to: [f32; 2], straight: bool) -> bool {
    let reached = straight && (to[0] - body.at[0]).hypot(to[1] - body.at[2]) < (body.size[0] * 0.5).max(NODE_REACHED);    world.get_mut::<Hunter>(entity).is_some_and(|mut hunter| std::mem::replace(&mut hunter.reached, reached))
}

/// Before the hunter's move without prey: it walks to `to` as it would at prey, by a way round
/// where one is needed, until it is `near` of it.
pub(in crate::creatures) fn approach(world: &mut World, entity: Entity, layer: Entity, body: &Body, to: [f32; 3], speed: f32, near: f32) {
    let (step, straight) = heads(world, entity, layer, body, to);
    legs::walk(world, entity, layer, body, step, speed, if straight { near } else { 0.0 });
}
