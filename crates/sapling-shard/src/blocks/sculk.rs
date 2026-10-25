//! Sculk sensors, calibrated sensors and shriekers hearing vibrations (docs/research/sculk.md): a
//! game event within a listener's range reaches it as many ticks later as it is blocks away, and
//! an idle sensor that hears it is active a while, powering what is around it the more the nearer
//! the event, then cools down. A calibrated sensor powered from behind hears only the frequency
//! of that power. A shrieker hears sensors going active, and shrieks when a player set one off.
// TODO: nothing occludes or dampens (wool); no amethyst resonates; no vibration particle; a
// shrieker warns no one and summons no warden; mobs, eating, damage and explosions are not heard.

mod events;

pub(crate) use events::GameEvent;

use std::collections::{HashMap, HashSet};

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};
use sapling_world::ChunkPos;

use super::queued::{self, Job};
use super::{level, redstone, send, with_property};
use crate::components::Tick;

const SENSOR: &str = "minecraft:sculk_sensor";
const CALIBRATED: &str = "minecraft:calibrated_sculk_sensor";
const SHRIEKER: &str = "minecraft:sculk_shrieker";
const PHASE: &str = "sculk_sensor_phase";
const RANGE: f32 = 8.0;
const CALIBRATED_RANGE: f32 = 16.0;
/// Ticks a sensor is active, a calibrated one, and cools down after.
const ACTIVE: u64 = 30;
const CALIBRATED_ACTIVE: u64 = 10;
const COOLDOWN: u64 = 10;
/// Ticks a shrieker shrieks.
const SHRIEK: u64 = 90;
const IDLE: u8 = 0;
const LISTENING: u8 = 1;
const COOLING: u8 = 2;
/// A step's share of a block walked (the Java game's `moveDist` of 0.6 a block, a step at each whole).
const STRIDE: f32 = 0.6;

/// A vibration on its way to a listener.
struct Coming {
    when: u64,
    layer: Entity,
    listener: BlockPos,
    frequency: u8,
    strength: u8,
    by_player: bool,
}

#[derive(Resource, Default)]
pub(crate) struct Listeners {
    /// The sensors and shriekers of each column, by world.
    listeners: HashMap<(Entity, ChunkPos), HashSet<BlockPos>>,
    coming: Vec<Coming>,
    /// The frequency each sensor last heard, which a comparator reads.
    heard: HashMap<(Entity, BlockPos), u8>,
    /// How far towards its next step each player has walked.
    strides: HashMap<Entity, f32>,
}

fn name_at(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Option<(&'static str, &'static str, u32)> {
    let level = level(world, layer).filter(|level| level.in_height(y) && level.is_loaded(send::column([x, y, z])))?;
    let id = level.block(x, y, z);
    let state = level.registry().get(id)?;
    Some((state.name, state.properties, id))
}

fn number(properties: &str, key: &str) -> Option<u8> {
    properties.split(',').find_map(|pair| pair.strip_prefix(key)?.strip_prefix('=')?.parse().ok())
}

fn is_sensor(name: &str) -> bool {
    name == SENSOR || name == CALIBRATED
}

fn is_listener(name: &str) -> bool {
    is_sensor(name) || name == SHRIEKER
}

/// A block was written at `pos`, `was` one block and `now` another: listeners are kept track of.
pub(super) fn written(world: &mut World, layer: Entity, pos: BlockPos, was: &str, now: &str) {
    if was == now || !(is_listener(was) || is_listener(now)) {
        return;
    }
    let mut listeners = world.get_resource_or_insert_with(Listeners::default);
    let column = listeners.listeners.entry((layer, send::column(pos))).or_default();
    if is_listener(now) {
        column.insert(pos);
    } else {
        column.remove(&pos);
        listeners.heard.remove(&(layer, pos));
    }
}

/// The listeners among the cells of a column that came from storage; one saved mid-phase, whose
/// queued step was not saved, steps on.
pub(super) fn found(world: &mut World, layer: Entity, cells: &[BlockPos]) {
    let busy = |properties: &str| number(properties, PHASE).unwrap_or(IDLE) != IDLE || number(properties, "active").unwrap_or(0) != 0;
    let found: Vec<(BlockPos, bool)> = cells.iter().filter_map(|pos| name_at(world, layer, *pos).filter(|(name, ..)| is_listener(name)).map(|(_, properties, _)| (*pos, busy(properties)))).collect();
    let Some((first, _)) = found.first() else { return };
    let column = send::column(*first);
    world.get_resource_or_insert_with(Listeners::default).listeners.entry((layer, column)).or_default().extend(found.iter().map(|(pos, _)| *pos));
    for (pos, _) in found.into_iter().filter(|(_, busy)| *busy) {
        queued::after(world, layer, pos, 1, Job::Sculk);
    }
}

/// What a comparator reads off the block at `pos`: the frequency an active sensor heard, else 0;
/// `None` for what is no sensor.
pub(crate) fn signal(world: &World, layer: Entity, pos: BlockPos) -> Option<u8> {
    let (_, properties, _) = name_at(world, layer, pos).filter(|(name, ..)| is_sensor(name))?;
    let heard = world.get_resource::<Listeners>().and_then(|listeners| listeners.heard.get(&(layer, pos)).copied());
    Some(heard.filter(|_| number(properties, PHASE) == Some(LISTENING)).unwrap_or(0))
}

/// The side a calibrated sensor takes its frequency on: behind the way it faces.
fn back(properties: &str) -> Face {
    match properties.split(',').find_map(|pair| pair.strip_prefix("minecraft:cardinal_direction=")) {
        Some("north") => Face::South,
        Some("south") => Face::North,
        Some("west") => Face::East,
        _ => Face::West,
    }
}

/// Whether the listener `name` at `pos` hears `event` from `at`, by a player or not (S5, S11, S16).
fn hears(world: &World, layer: Entity, (name, properties): (&str, &str), pos: BlockPos, at: BlockPos, event: GameEvent, by_player: bool) -> bool {
    if name == SHRIEKER {
        return event == GameEvent::TendrilsClicking && by_player && number(properties, "active") == Some(0);
    }
    let own = matches!(event, GameEvent::BlockPlace | GameEvent::BlockDestroy) && at == pos || event == GameEvent::SculkTouch && at != pos;
    if event.frequency() == 0 || own || number(properties, PHASE).unwrap_or(IDLE) != IDLE {
        return false;
    }
    let tuned = if name == CALIBRATED { redstone::power_at(world, layer, back(properties).neighbour(pos)) } else { 0 };
    tuned == 0 || tuned == event.frequency()
}

/// `event` happened at `at`, by a player or not: each listener in range that hears it starts to
/// wait for it, unless one is already on its way to it (S3-S8).
pub(crate) fn vibrate(world: &mut World, layer: Entity, at: [f32; 3], event: GameEvent, by_player: bool) {
    let Some(listeners) = world.get_resource::<Listeners>() else { return };
    let reach = CALIBRATED_RANGE as i32;
    let corner = |dx: i32, dz: i32| send::column([at[0] as i32 + dx, 0, at[2] as i32 + dz]);
    let (low, high) = (corner(-reach, -reach), corner(reach, reach));
    let near: Vec<BlockPos> = (low.x..=high.x)
        .flat_map(|x| (low.z..=high.z).map(move |z| ChunkPos { x, z }))
        .filter_map(|column| listeners.listeners.get(&(layer, column)))
        .flatten()
        .copied()
        .collect();
    if near.is_empty() {
        return;
    }
    let now = world.resource::<Tick>().0;
    let cell = at.map(|c| c.floor() as i32);
    for listener in near {
        let Some((name, properties, _)) = name_at(world, layer, listener) else { continue };
        let range = if name == CALIBRATED { CALIBRATED_RANGE } else { RANGE };
        let distance = (0..3).map(|axis| ((listener[axis] - cell[axis]) as f32).powi(2)).sum::<f32>().sqrt();
        if distance > range || !hears(world, layer, (name, properties), listener, cell, event, by_player) {
            continue;
        }
        let strength = (15 - (15.0 * distance / range).floor() as i32).clamp(1, 15) as u8;
        let mut listeners = world.resource_mut::<Listeners>();
        if listeners.coming.iter().any(|coming| (coming.layer, coming.listener) == (layer, listener)) {
            continue;
        }
        let when = now + (distance as u64).max(1);
        listeners.coming.push(Coming { when, layer, listener, frequency: event.frequency(), strength, by_player });
    }
}

/// [`vibrate`] from the middle of the block at `pos`.
pub(crate) fn vibrate_at(world: &mut World, layer: Entity, pos: BlockPos, event: GameEvent, by_player: bool) {
    vibrate(world, layer, pos.map(|c| c as f32 + 0.5), event, by_player);
}

/// A player walked from `from` to `to` (feet): a step is heard every 1/0.6 blocks walked on the
/// ground, but not sneaking; a sensor walked on is touched, sneaking or not (S14).
pub(crate) fn walked(world: &mut World, player: Entity, layer: Entity, from: [f32; 3], to: [f32; 3], on_ground: bool, sneaking: bool) {
    if !on_ground {
        return;
    }
    let under = [to[0].floor() as i32, (to[1] - 0.5).floor() as i32, to[2].floor() as i32];
    if name_at(world, layer, under).is_some_and(|(name, ..)| is_sensor(name)) {
        vibrate_at(world, layer, under, GameEvent::SculkTouch, true);
    }
    if sneaking {
        return;
    }
    let walked = (to[0] - from[0]).hypot(to[2] - from[2]) * STRIDE;
    let mut listeners = world.get_resource_or_insert_with(Listeners::default);
    let gone = listeners.strides.entry(player).or_insert(0.0);
    *gone += walked;
    let stepped = *gone >= 1.0;
    *gone = gone.fract();
    if stepped {
        vibrate(world, layer, to, GameEvent::Step, true);
    }
}

/// Listeners that a vibration reached this tick act on it.
pub(crate) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let Some(mut listeners) = world.get_resource_mut::<Listeners>().filter(|listeners| !listeners.coming.is_empty()) else { return };
    let (due, waiting): (Vec<Coming>, Vec<Coming>) = std::mem::take(&mut listeners.coming).into_iter().partition(|coming| coming.when <= now);
    listeners.coming = waiting;
    for coming in due {
        match name_at(world, coming.layer, coming.listener) {
            Some((SHRIEKER, ..)) => shriek(world, coming.layer, coming.listener),
            Some((name, ..)) if is_sensor(name) => activate(world, coming),
            _ => {}
        }
    }
}

fn set_number(world: &mut World, layer: Entity, pos: BlockPos, key: &str, value: u8) -> bool {
    let Some((_, _, id)) = name_at(world, layer, pos) else { return false };
    let Some(next) = level(world, layer).and_then(|level| with_property(level.registry(), id, key, &value.to_string())) else { return false };
    send::set(world, layer, pos, next)
}

fn play(world: &mut World, layer: Entity, pos: BlockPos, sound: &str) {
    let Some(level) = level(world, layer) else { return };
    let noise = send::sound(sound, level.registry(), pos, level.block(pos[0], pos[1], pos[2]));
    send::to_viewers(world, layer, pos, &[noise]);
}

/// A vibration reached an idle sensor: it goes active, powered by how near it was (S9-S10).
fn activate(world: &mut World, Coming { layer, listener, frequency, strength, by_player, .. }: Coming) {
    let Some((name, properties, _)) = name_at(world, layer, listener) else { return };
    if number(properties, PHASE).unwrap_or(IDLE) != IDLE {
        return;
    }
    set_number(world, layer, listener, PHASE, LISTENING);
    redstone::emit(world, layer, listener, strength);
    world.get_resource_or_insert_with(Listeners::default).heard.insert((layer, listener), frequency);
    super::container_changed(world, layer, listener);
    play(world, layer, listener, "power.on.sculk_sensor");
    queued::cancel(world, layer, listener, Job::Sculk);
    queued::after(world, layer, listener, if name == CALIBRATED { CALIBRATED_ACTIVE } else { ACTIVE }, Job::Sculk);
    vibrate_at(world, layer, listener, GameEvent::TendrilsClicking, by_player);
}

/// A shrieker that heard a sensor a player set off shrieks (S17-S19).
fn shriek(world: &mut World, layer: Entity, pos: BlockPos) {
    if !name_at(world, layer, pos).is_some_and(|(_, properties, _)| number(properties, "active") == Some(0)) {
        return;
    }
    set_number(world, layer, pos, "active", 1);
    queued::after(world, layer, pos, SHRIEK, Job::Sculk);
    let position = Vec3f { x: pos[0] as f32, y: pos[1] as f32, z: pos[2] as f32 };
    let shrieked = LevelEvent { event: LevelEventEvent::ParticleSculkShriek, position, data: 0 };
    send::to_viewers(world, layer, pos, &[sapling_net::encode(&shrieked)]);
    play(world, layer, pos, "shriek");
}

/// A sensor's or shrieker's time in its phase is up: an active sensor cools down, a cooling one
/// listens again, a shrieker falls quiet.
pub(super) fn step(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some((name, properties, _)) = name_at(world, layer, pos) else { return };
    if name == SHRIEKER {
        set_number(world, layer, pos, "active", 0);
        return;
    }
    match number(properties, PHASE) {
        Some(LISTENING) if is_sensor(name) => {
            set_number(world, layer, pos, PHASE, COOLING);
            redstone::emit(world, layer, pos, 0);
            super::container_changed(world, layer, pos);
            queued::after(world, layer, pos, COOLDOWN, Job::Sculk);
        }
        Some(COOLING) if is_sensor(name) => {
            set_number(world, layer, pos, PHASE, IDLE);
            play(world, layer, pos, "power.off.sculk_sensor");
        }
        _ => {}
    }
}
