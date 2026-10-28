//! Minecarts: placed on a rail from their item, rolled along rails (rails.rs) and falling off
//! them as any body does, pushed by those who walk into them; a detector rail is pressed under
//! one, an activator rail throws out a minecart's rider and primes a TNT minecart, which bursts at
//! the end of its fuse or when it falls hard (docs/research/vehicles.md "Minecarts").
// TODO: a TNT minecart's blast is not scaled by its speed and nothing but flint and steel, an
// activator rail and a fall sets one off; a cart's yaw does not follow its rail.

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use super::rails::{self, Rail, RailKind, Roll};
use super::{MINECART, TNT_MINECART, is_crowded, spawn};
use crate::body::{Body, InLiquid, Physics};
use crate::components::{InLayer, Layer, NetId, Position, Tick};
use crate::creatures;
use crate::entities::{self, Kind};
use crate::riding::{Reins, Riding};
use crate::sync::Told;

/// The wire puts a minecart this far above its box's bottom.
const LIFT: f32 = 0.35;
/// Off a rail a cart falls as a block does and slides half as far on the ground.
const LOOSE: Physics = Physics { gravity: 0.04, air_drag: 0.95, fall_drag: 0.95, ground_drag: 0.5, bounce: 0.0, liquid: InLiquid::Sinks, burns: false, passes: false, water_drag: 0.8 };
const RIDDEN_DRAG: f32 = 0.997;
const EMPTY_DRAG: f32 = 0.96;
/// A cart with goods keeps more of its motion, the less full it is: its comparator signal.
// TODO: the signal of what it carries is taken as nothing.
const LADEN_DRAG: f32 = 0.98 + 15.0 * 0.001;
pub(super) const FUSE: i32 = 80;
/// The fuse of one a blast reached: from this many ticks, to this many more.
pub(super) const SHORT_FUSE: [i32; 2] = [10, 20];
const BLAST: f32 = 3.0;
/// A TNT minecart falling further than this bursts as it lands.
const HARD_FALL: f32 = 3.5;
/// A detector rail stays pressed this long after a cart was last on it.
const DETECTED: u64 = 20;
/// Others push a cart from this near, by this much (BDS `legacy_minecart` push).
const NEAR: f32 = 0.2;
const PUSH: f32 = 0.1;
const PUSHED_SHARE: f32 = 0.5;
const PUSHER_SHARE: f32 = 0.25;

/// A minecart's own part: the fuse of a primed TNT minecart, and how far it fell.
#[derive(Component, Clone, Copy, Default)]
pub(super) struct Cart {
    fuse: Option<i32>,
    fallen: f32,
}

/// The detector rails carts pressed, with the last tick one was on each.
#[derive(Resource, Default)]
struct Pressed(HashMap<(Entity, BlockPos), u64>);

pub(super) fn fit(world: &mut World, cart: Entity) {
    world.entity_mut(cart).insert(Cart::default());
}

/// Places a minecart of `kind` on the rail at `pos`; false off a rail or where a mob or vehicle is
/// in the rail's cell (BDS `MinecartItem::_useOn`).
pub(super) fn place(world: &mut World, layer: Entity, pos: BlockPos, kind: &str) -> bool {
    let Some(rail) = world.get::<Layer>(layer).and_then(|layer| rails::rail_at(&layer.world, pos)) else { return false };
    let [x, y, z] = rail.pos.map(|c| c as f32);
    if is_crowded(world, layer, ([x, y, z], [x + 1.0, y + 1.0, z + 1.0])) {
        return false;
    }
    spawn(world, layer, kind, [x + 0.5, y, z + 0.5], 0.0, 0, None).is_some()
}

/// Primes a TNT minecart to burst in `fuse` ticks; false for any other vehicle, or one primed.
pub(super) fn prime(world: &mut World, cart: Entity, fuse: i32) -> bool {
    let is_tnt = world.get::<Kind>(cart).is_some_and(|kind| kind.0 == TNT_MINECART);
    let Some(mut state) = world.get_mut::<Cart>(cart).filter(|state| is_tnt && state.fuse.is_none()) else { return false };
    state.fuse = Some(fuse);
    true
}

/// How a cart rolls this tick: its drag, its rider's push, the solid blocks at its rail's ends.
fn how(world: &World, cart: Entity, kind: &str, rail: &Rail, level: &sapling_world::World) -> Roll {
    let driver = world.get::<crate::riding::Seats>(cart).and_then(|seats| seats.0.first().copied());
    let pushed_to = driver.filter(|driver| world.get::<Reins>(*driver).is_some_and(|reins| reins.keys[1] > 0.0)).and_then(|driver| world.get::<Position>(driver)).map(|at| at.yaw);
    let drag = if driver.is_some() { RIDDEN_DRAG } else if kind == MINECART || kind == TNT_MINECART { EMPTY_DRAG } else { LADEN_DRAG };
    let solid = |face: Face| {
        let [x, y, z] = face.neighbour(rail.pos);
        level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_full_cube())
    };
    Roll { drag, ridden: driver.is_some(), pushed_to, walls: [solid(Face::North), solid(Face::South), solid(Face::West), solid(Face::East)] }
}

/// Moves each minecart a tick, and lets its rail act on it.
pub(super) fn tick(world: &mut World) {
    push(world);
    let carts: Vec<(Entity, Entity, u64, String, Body)> = world.query_filtered::<(Entity, &InLayer, &NetId, &Kind, &Body), With<Cart>>().iter(world).map(|(cart, layer, id, kind, body)| (cart, layer.0, id.0, kind.0.clone(), *body)).collect();
    for (cart, layer, id, kind, body) in carts {
        let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { continue };
        let on = rails::rail_under(level, body.at);
        let (body, packets) = match on {
            Some(rail) => {
                let (feet, motion) = rails::roll(&rail, body.at, body.motion, &how(world, cart, &kind, &rail, level));
                let Some(mut moved) = world.get_mut::<Body>(cart) else { continue };
                (moved.at, moved.motion, moved.on_ground) = (feet, motion, true);
                let moved = *moved;
                let Some(mut told) = world.get_mut::<Told>(cart) else { continue };
                let packets = told.update(id, [feet[0], feet[1] + LIFT, feet[2]], motion, true);
                act(world, cart, layer, &kind, &rail);
                (moved, packets)
            }
            None => {
                let Some((moved, packets)) = entities::step(world, cart, layer, id, &LOOSE, LIFT) else { continue };
                (moved, packets)
            }
        };
        entities::tell_viewers(world, cart, layer, body.at.map(|c| c.floor() as i32), packets);
        // Read after the rail acted: an activator rail may have primed it.
        let Some(mut state) = world.get::<Cart>(cart).copied() else { continue };
        let landed_hard = kind == TNT_MINECART && body.on_ground && state.fallen > HARD_FALL;
        state.fallen = if body.on_ground { 0.0 } else { state.fallen - body.motion[1].min(0.0) };
        state.fuse = state.fuse.map(|fuse| fuse - 1);
        if landed_hard || state.fuse.is_some_and(|fuse| fuse <= 0) {
            crate::riding::forget(world, cart);
            creatures::explode(world, Some(cart), layer, body.at, BLAST, crate::tnt::wreck(world, layer));
            continue;
        }
        if let Ok(mut entity) = world.get_entity_mut(cart) {
            entity.insert(state);
        }
    }
    release_detectors(world);
}

/// What the rail under a cart does to it: a detector rail is pressed; a powered activator rail
/// throws out a minecart's riders and primes a TNT minecart.
fn act(world: &mut World, cart: Entity, layer: Entity, kind: &str, rail: &Rail) {
    match rail.kind {
        RailKind::Detector => {
            let now = world.resource::<Tick>().0;
            world.get_resource_or_insert_with(Pressed::default).0.insert((layer, rail.pos), now);
            if !rail.on {
                set_pressed(world, layer, rail.pos, true);
            }
        }
        RailKind::Activator if rail.on && kind == MINECART => drop(crate::riding::evict(world, cart)),
        RailKind::Activator if rail.on => drop(prime(world, cart, FUSE)),
        _ => {}
    }
}

fn set_pressed(world: &mut World, layer: Entity, [x, y, z]: BlockPos, on: bool) {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    if let Some(id) = crate::blocks::with_property(level.registry(), level.block(x, y, z), "rail_data_bit", if on { "1" } else { "0" }) {
        crate::blocks::set(world, layer, [x, y, z], id);
    }
}

/// Lets go of the detector rails no cart was on for a while.
fn release_detectors(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let Some(mut pressed) = world.get_resource_mut::<Pressed>() else { return };
    let gone: Vec<(Entity, BlockPos)> = pressed.0.iter().filter(|(_, last)| now >= **last + DETECTED).map(|(key, _)| *key).collect();
    pressed.0.retain(|key, _| !gone.contains(key));
    for (layer, pos) in gone {
        if world.get::<Layer>(layer).and_then(|level| rails::rail_at(&level.world, pos)).is_some_and(|rail| rail.kind == RailKind::Detector && rail.on) {
            set_pressed(world, layer, pos, false);
        }
    }
}

/// Mobs and players walking into a cart push it, and a mob is pushed back.
fn push(world: &mut World) {
    let carts: Vec<(Entity, Entity, Body)> = world.query_filtered::<(Entity, &InLayer, &Body), With<Cart>>().iter(world).map(|(cart, layer, body)| (cart, layer.0, *body)).collect();
    if carts.is_empty() {
        return;
    }
    let mut others: Vec<(Option<Entity>, Entity, [f32; 3], [f32; 2])> =
        world.query_filtered::<(Entity, &InLayer, &Body), (With<creatures::Creature>, Without<Riding>)>().iter(world).map(|(mob, layer, body)| (Some(mob), layer.0, body.at, body.size)).collect();
    others.extend(world.query_filtered::<(&InLayer, &Position), Without<Riding>>().iter(world).map(|(layer, at)| (None, layer.0, at.feet, [0.6, 1.8])));
    for (cart, layer, body) in carts {
        let (low, high) = body.bounds();
        for (mob, other_layer, at, [width, height]) in &others {
            let touches = *other_layer == layer && (at[0] - width / 2.0 < high[0] + NEAR && at[0] + width / 2.0 > low[0] - NEAR) && (at[2] - width / 2.0 < high[2] + NEAR && at[2] + width / 2.0 > low[2] - NEAR) && at[1] < high[1] && at[1] + height > low[1];
            let (dx, dz) = (at[0] - body.at[0], at[2] - body.at[2]);
            let far = dx.hypot(dz);
            if !touches || far < 0.01 {
                continue;
            }
            let force = (1.0 / far).min(1.0) * PUSH;
            let away = [dx / far * force, dz / far * force];
            if let Some(mut pushed) = world.get_mut::<Body>(cart) {
                (pushed.motion[0], pushed.motion[2]) = (pushed.motion[0] - away[0] * PUSHED_SHARE, pushed.motion[2] - away[1] * PUSHED_SHARE);
            }
            if let Some(mut pusher) = mob.and_then(|mob| world.get_mut::<Body>(mob)) {
                (pusher.motion[0], pusher.motion[2]) = (pusher.motion[0] + away[0] * PUSHER_SHARE, pusher.motion[2] + away[1] * PUSHER_SHARE);
            }
        }
    }
}
