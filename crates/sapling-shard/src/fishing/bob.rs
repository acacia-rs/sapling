//! A hook's tick, by BDS's `FishingHook::normalTick` (1429c8960): in lava it comes back; it falls
//! 0.04 a tick out of water and rises by how much of it is under, held at the surface by a bob;
//! its motion keeps 0.92 (0.5 on the ground or against a wall, 0.9 of that in water). It then
//! moves by that twice, keeping 0.99 and falling 0.05 (fitted to research/captures/bds-fish.txt:
//! exact in the air and the first tick in water). In water fish come (bite.rs). What it meets
//! first on its way it hooks, and then sits in; a move cut short by a block leaves it in the
//! ground until one is not.
// TODO: in water the fit drifts by about 1e-3 a tick from the third tick on.

use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValueDefault as Plain};
use bevy_ecs::prelude::*;
use sapling_world::World as Level;

use super::{Hook, bite, reel};
use crate::body::{Body, Liquid, liquid_in, surface};
use crate::components::{InLayer, Layer, NetId, Position};
use crate::creatures::{Creature, Health};
use crate::dragon::cloud::item;
use crate::entities;
use crate::sync::Told;

const FRICTION: f32 = 0.92;
const STOPPED: f32 = 0.5;
const WET: f32 = 0.9;
const BOB_STEP: f32 = 0.1;
const BOB: f32 = 0.035;
const SINK: f32 = 0.04;
const UNDER: f32 = 1.01;
const LIFT: f32 = 0.15;
/// The projectile's own move: what it keeps of its motion, and its fall.
const DRAG: f32 = 0.99;
const GRAVITY: f32 = 0.05;
/// Bodies are hooked this far outside their boxes.
const GRACE: f32 = 0.3;
/// A hooked body holds the hook this far up its height.
const HELD: f32 = 0.8;
const SLICES: usize = 5;
const PLAYER: [f32; 2] = [0.6, 1.8];

/// How much of the hook's box, in five slices, is under water.
fn wet_part(level: &Level, body: &Body) -> f32 {
    let (low, high) = body.bounds();
    let cell = |y: f32| [body.at[0].floor() as i32, y.floor() as i32, body.at[2].floor() as i32];
    let wet = (0..SLICES).filter(|slice| {
        let bottom = low[1] + (high[1] - low[1]) * *slice as f32 / SLICES as f32;
        let at = cell(bottom);
        liquid_in(level, at, Liquid::Water).is_some_and(|found| bottom < surface(at[1], found))
    });
    wet.count() as f32 / SLICES as f32
}

/// The bodies a hook may catch: mobs alive and players but its fisher, each with its box.
fn bodies(world: &mut World, layer: Entity, fisher: Entity) -> Vec<(Entity, [f32; 3], [f32; 2])> {
    let mut mobs = world.query_filtered::<(Entity, &InLayer, &Body), (With<Creature>, With<Health>)>();
    let mut found: Vec<_> = mobs.iter(world).filter(|(_, in_layer, _)| in_layer.0 == layer).map(|(mob, _, body)| (mob, body.at, body.size)).collect();
    let mut players = world.query::<(Entity, &InLayer, &Position)>();
    found.extend(players.iter(world).filter(|(player, in_layer, _)| in_layer.0 == layer && *player != fisher).map(|(player, _, position)| (player, position.feet, PLAYER)));
    found
}

/// Where along `motion` from `at` (0 to 1) a box of `size` at `feet`, grown by [`GRACE`], is met.
fn met(at: [f32; 3], motion: [f32; 3], feet: [f32; 3], size: [f32; 2]) -> Option<f32> {
    let half = size[0] / 2.0 + GRACE;
    let low = [feet[0] - half, feet[1] - GRACE, feet[2] - half];
    let high = [feet[0] + half, feet[1] + size[1] + GRACE, feet[2] + half];
    let (mut enter, mut leave) = (0.0f32, 1.0f32);
    for axis in 0..3 {
        if motion[axis].abs() < 1e-7 {
            if at[axis] < low[axis] || at[axis] > high[axis] {
                return None;
            }
            continue;
        }
        let [a, b] = [(low[axis] - at[axis]) / motion[axis], (high[axis] - at[axis]) / motion[axis]];
        (enter, leave) = (enter.max(a.min(b)), leave.min(a.max(b)));
    }
    (enter <= leave).then_some(enter)
}

/// The nearest body the hook meets on its way this tick.
fn first_met(world: &mut World, layer: Entity, fisher: Entity, at: [f32; 3], motion: [f32; 3]) -> Option<Entity> {
    let found = bodies(world, layer, fisher).into_iter().filter_map(|(body, feet, size)| Some((met(at, motion, feet, size)?, body)));
    found.min_by(|a, b| a.0.total_cmp(&b.0)).map(|(_, body)| body)
}

/// Where a hooked player or mob stands and its size, while it lives.
pub(super) fn alive(world: &World, target: Entity) -> Option<([f32; 3], [f32; 2])> {
    match world.get::<Position>(target) {
        Some(position) => (!crate::vitals::is_dead(world, target)).then_some((position.feet, PLAYER)),
        None => world.get::<Body>(target).filter(|_| crate::creatures::is_alive(world, target)).map(|body| (body.at, body.size)),
    }
}

/// Sits the hook on what it hooked; false when that is gone.
fn sit(world: &mut World, hook: Entity, target: Entity, id: u64, layer: Entity) -> bool {
    let Some((feet, size)) = alive(world, target) else { return false };
    let at = [feet[0], feet[1] + HELD * size[1], feet[2]];
    let Some(mut body) = world.get_mut::<Body>(hook) else { return false };
    (body.at, body.motion) = (at, [0.0; 3]);
    let packets = world.get_mut::<Told>(hook).map(|mut told| told.update(id, at, [0.0; 3], false)).unwrap_or_default();
    entities::tell_viewers(world, hook, layer, at.map(|c| c.floor() as i32), packets);
    true
}

/// One tick of the hook.
pub(super) fn tick(world: &mut World, hook: Entity, layer: Entity) {
    let (Some(mut body), Some(id), Some(state)) = (world.get::<Body>(hook).copied(), world.get::<NetId>(hook).map(|id| id.0), world.get::<Hook>(hook)) else { return };
    let (fisher, target) = (state.fisher, state.target);
    if let Some(target) = target {
        if !sit(world, hook, target, id, layer) {
            world.get_mut::<Hook>(hook).expect("read above").target = None;
        }
        return;
    }
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let cell = body.at.map(|c| c.floor() as i32);
    if liquid_in(level, cell, Liquid::Lava).is_some() {
        return reel::retrieve(world, fisher, hook);
    }
    let mut friction = if body.on_ground || touches_wall(&body) { STOPPED } else { FRICTION };
    let water = liquid_in(level, cell, Liquid::Water).map(|_| liquid_in(level, [cell[0], cell[1] + 1, cell[2]], Liquid::Water).is_some());
    let wet = wet_part(level, &body);
    let bob = {
        let mut state = world.get_mut::<Hook>(hook).expect("read above");
        state.bob += BOB_STEP;
        state.bob
    };
    let (depth, lift) = match water {
        Some(true) => (UNDER, 0.0),
        Some(false) => ((cell[1] + 1) as f32 - body.at[1], (bob.sin() + 1.0) * BOB),
        None => (0.0, 0.0),
    };
    let mut motion = body.motion;
    motion[1] += (2.0 * wet - 1.0) * SINK;
    if wet > 0.0 {
        motion[1] += bite::tick(world, hook, layer, &body);
        friction *= WET;
        motion[1] = ((depth - lift) * LIFT).min(motion[1] * 0.8 + 0.05);
    }
    let motion = motion.map(|c| c * friction);
    let in_ground = world.get::<Hook>(hook).is_some_and(|state| state.in_ground);
    if !in_ground && let Some(caught) = first_met(world, layer, fisher, body.at, motion) {
        hooked(world, hook, caught, layer);
        return;
    }
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    // BDS moves a hook twice a tick: its own tick, then the projectile's, which drags and drops it;
    // a block in the way of the first stops it dead, with no second.
    let stopped = (0..2).any(|_| {
        let before = body.at;
        body.shove(level, motion);
        let short = (0..3).any(|axis| (body.at[axis] - before[axis] - motion[axis]).abs() > 1e-4);
        body.on_ground = short && motion[1] < 0.0 && body.at[1] - before[1] > motion[1] + 1e-4;
        short
    });
    body.motion = if stopped { [0.0; 3] } else { [motion[0] * DRAG, motion[1] * DRAG - GRAVITY, motion[2] * DRAG] };
    // BDS's next tick does not count the landing as ground: it keeps 0.92 (bds-fish.txt).
    body.on_ground &= !stopped;
    world.get_mut::<Hook>(hook).expect("read above").in_ground = stopped;
    *world.get_mut::<Body>(hook).expect("read above") = body;
    let packets = world.get_mut::<Told>(hook).map(|mut told| told.update(id, body.at, body.motion, body.on_ground)).unwrap_or_default();
    entities::tell_viewers(world, hook, layer, body.at.map(|c| c.floor() as i32), packets);
}

// TODO: BDS reads the hook's horizontal-collision flag; a body keeps none, so a wall slows it only once on the ground.
fn touches_wall(_body: &Body) -> bool {
    false
}

/// The hook catches `caught`: its viewers are told whom it holds.
fn hooked(world: &mut World, hook: Entity, caught: Entity, layer: Entity) {
    let (Some(id), Some(target)) = (world.get::<NetId>(hook).map(|id| id.0), world.get::<NetId>(caught).map(|id| id.0)) else { return };
    world.get_mut::<Hook>(hook).expect("a hook").target = Some(caught);
    let at = world.get::<Body>(hook).map_or([0.0; 3], |body| body.at);
    let data = acacia_proto::packets::SetEntityData {
        runtime_entity_id: id,
        metadata: vec![item(Key::TargetEid, Plain::Long(target as i64))],
        properties: acacia_proto::types::EntityProperties { ints: Vec::new(), floats: Vec::new() },
        tick: 0,
    };
    entities::tell_viewers(world, hook, layer, at.map(|c| c.floor() as i32), vec![sapling_net::encode(&data)]);
}
