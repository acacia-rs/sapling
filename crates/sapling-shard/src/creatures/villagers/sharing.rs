//! A villager sharing food (its pack's `behavior.share_items` over its `minecraft:shareables`): one
//! holding more than an item's `surplus_amount` walks to a villager within 3 that holds less than
//! its `want_amount` and throws it the difference. BDS (bds-g11share.txt) threw 3 bread a time
//! every 1.5 s, 0.15 to 0.18 out of the thrower towards its mate, from 1.46 up, at 0.07 to 0.12
//! across and 0.0585 up; the thrower mostly took each back 0.45 s later.

use bevy_ecs::prelude::*;
use sapling_world::ItemStack;

use super::pantry::Pantry;
use super::{Trader, families};
use crate::body::Body;
use crate::components::{InLayer, Tick};

/// Each shared item: its `surplus_amount`, and its `want_amount` for a peasant and for any other.
const SHARED: [(&str, u32, [u32; 2]); 4] = [
    ("minecraft:bread", 6, [3, 3]),
    ("minecraft:carrot", 24, [60, 12]),
    ("minecraft:potato", 24, [60, 12]),
    ("minecraft:beetroot", 24, [60, 12]),
];
/// `max_dist`, `goal_radius`, and `minecraft:movement` 0.5 times `speed_multiplier` 0.5.
const SEES: f32 = 3.0;
const GIVES_WITHIN: f32 = 2.0;
const WALKS: f32 = 0.25;
const THROWS_EVERY: u64 = 30;
const FROM_UP: f32 = 1.4616;
const FROM_OUT: f32 = 0.16;
const ACROSS: f32 = 0.08;
const UP: f32 = 0.0585;

/// The tick a villager may next throw.
#[derive(Component)]
pub(super) struct Shared(u64);

/// How many of `name` the villager `entity` wants more.
fn wants(world: &World, entity: Entity, name: &str) -> u32 {
    let Some(&(_, _, [peasant, other])) = SHARED.iter().find(|(shared, ..)| *shared == name) else { return 0 };
    let want = if families::of(world, entity).is_some_and(|families| families.contains(&"peasant")) { peasant } else { other };
    want.saturating_sub(world.get::<Pantry>(entity).map_or(0, |pantry| pantry.count(name)))
}

/// What the villager has a surplus of, and the nearest other villager within [`SEES`] wanting it.
fn pick(world: &mut World, entity: Entity, layer: Entity, body: &Body) -> Option<(&'static str, Entity, [f32; 3])> {
    let pantry = world.get::<Pantry>(entity)?;
    let surplus: Vec<&'static str> = SHARED.iter().filter(|(name, surplus, _)| pantry.count(name) > *surplus).map(|(name, ..)| *name).collect();
    if surplus.is_empty() {
        return None;
    }
    let off = |at: [f32; 3]| (0..3).map(|axis| (at[axis] - body.at[axis]).powi(2)).sum::<f32>().sqrt();
    let mut near: Vec<(Entity, [f32; 3])> = world
        .query::<(Entity, &Trader, &Body, &InLayer)>()
        .iter(world)
        .filter(|(other, trader, at, on)| *other != entity && on.0 == layer && trader.leaves_at.is_none() && off(at.at) <= SEES)
        .map(|(other, _, at, _)| (other, at.at))
        .collect();
    near.sort_by(|a, b| off(a.1).total_cmp(&off(b.1)));
    near.into_iter().find_map(|(other, at)| surplus.iter().find(|name| wants(world, other, name) > 0).map(|name| (*name, other, at)))
}

/// Before its move: a grown villager with food to spare walks to a villager wanting it and throws
/// it some. False with none to share or none near wanting.
pub(super) fn share(world: &mut World, entity: Entity, layer: Entity, body: &Body) -> bool {
    let now = world.resource::<Tick>().0;
    if crate::creatures::is_baby(world, entity) || world.get::<Shared>(entity).is_some_and(|shared| now < shared.0) {
        return false;
    }
    let Some((name, other, to)) = pick(world, entity, layer, body) else { return false };
    let (dx, dz) = (to[0] - body.at[0], to[2] - body.at[2]);
    let apart = dx.hypot(dz);
    if apart > GIVES_WITHIN {
        crate::creatures::legs::walk(world, entity, layer, body, [to[0], to[2]], WALKS, GIVES_WITHIN);
        return true;
    }
    let count = wants(world, other, name);
    let Some(taken) = world.get_mut::<Pantry>(entity).map(|mut pantry| pantry.take(name, count)) else { return false };
    let (x, z) = if apart > 0.0 { (dx / apart, dz / apart) } else { (1.0, 0.0) };
    let from = [body.at[0] + x * FROM_OUT, body.at[1] + FROM_UP, body.at[2] + z * FROM_OUT];
    let stack = ItemStack { name: name.into(), count: taken as u8, damage: 0, block: None, tag: None };
    crate::drops::hand_on(world, layer, stack, from, [x * ACROSS, UP, z * ACROSS]);
    world.entity_mut(entity).insert(Shared(now + THROWS_EVERY));
    crate::creatures::legs::rest(world, entity);
    true
}
