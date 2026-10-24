//! A tick of each living dragon of an End (`EnderDragon::aiStep` 145994090 and its goals,
//! research/dragon-ai-decompile.md 3): its stored motion carries it, its goal steers, then its
//! wings are heard, it growls now and then in flight, heals from the crystal it found, looks for
//! one now and then, and flies; its parts hit and break what they touch (parts.rs); its viewers
//! are told (wire.rs).

use acacia_proto::packets::{EntityEvent, EntityEventEventId, LevelSoundEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;

use super::flight::{self, Ctx, Flight};
use super::{Crystal, Dragon, Fight, parts, wire};
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Tick};
use crate::creatures::Health;

const HEAD_UP: f32 = 2.0;
const HEAL_EVERY: u64 = 10;
const SEARCH_ODDS: u32 = 10;
const CRYSTAL_REACH: f32 = 32.0;

/// A sound of the dragon's at `at`.
pub(crate) fn sound(name: &str, id: u64, [x, y, z]: [f32; 3]) -> LevelSoundEvent {
    LevelSoundEvent { sound_id: name.into(), position: Vec3f { x, y, z }, extra_data: -1, entity_type: super::DRAGON.into(), is_baby_mob: false, is_global: false, entity_unique_id: id as i64, fire_at_position: None }
}

fn crystals_of(world: &World, layer: Entity) -> u32 {
    world.get::<Fight>(layer).map_or(0, |fight| fight.crystals)
}

/// The nearest crystal within 32 of `at`, the one it heals from.
fn healer_near(world: &mut World, layer: Entity, at: [f32; 3]) -> Option<Entity> {
    let mut crystals = world.query_filtered::<(Entity, &InLayer, &Body), With<Crystal>>();
    let far = |body: &Body| (0..3).map(|axis| (body.at[axis] - at[axis]).powi(2)).sum::<f32>();
    crystals.iter(world).filter(|(_, in_layer, body)| in_layer.0 == layer && far(body) < CRYSTAL_REACH * CRYSTAL_REACH).min_by(|a, b| far(a.2).total_cmp(&far(b.2))).map(|(crystal, ..)| crystal)
}

/// Heals it a point every ten of its ticks from a crystal it has found, and looks for one a tick
/// in ten.
fn heal(world: &mut World, layer: Entity, dragon: Entity, flight: &mut Flight, random: &mut super::random::Random, age: u64) {
    if flight.healer.is_some_and(|crystal| world.get::<Crystal>(crystal).is_some()) && age % HEAL_EVERY == 0 {
        if let Some(mut health) = world.get_mut::<Health>(dragon).filter(|health| health.left() < health.max()) {
            health.mend(1.0);
        }
    }
    if random.next_int(SEARCH_ODDS) == 0 {
        flight.healer = healer_near(world, layer, flight.motion.at);
    }
}

/// A tick of each living dragon of `layer`.
pub(super) fn tick(world: &mut World, layer: Entity) {
    let dragons: Vec<Entity> = world.query_filtered::<(Entity, &InLayer), With<Dragon>>().iter(world).filter(|(_, in_layer)| in_layer.0 == layer).map(|(dragon, _)| dragon).collect();
    for dragon in dragons {
        if super::is_dying(world, dragon) {
            continue;
        }
        let Some(mut kept) = world.entity_mut(dragon).take::<Dragon>() else { continue };
        let Some(mut random) = world.get::<Fight>(layer).map(|fight| fight.random) else {
            world.entity_mut(dragon).insert(kept);
            continue;
        };
        let dead = step(world, layer, dragon, &mut kept, &mut random);
        world.entity_mut(dragon).insert(kept);
        if let Some(mut fight) = world.get_mut::<Fight>(layer) {
            fight.random = random;
        }
        if dead {
            super::hooks::killed_at_portal(world, dragon);
        }
    }
}

/// One tick of `dragon`; whether its last flight ended.
fn step(world: &mut World, layer: Entity, dragon: Entity, kept: &mut Dragon, random: &mut super::random::Random) -> bool {
    let flight = &mut kept.flight;
    // creatures::tick passes the dragon by, so its guard runs down and its health is told here.
    let health_told = world.get_mut::<Health>(dragon).and_then(|mut health| {
        health.begin();
        health.untold()
    });
    if let Some(mut level) = world.get_mut::<Layer>(layer) {
        flight.keep_nodes(&mut level.world);
    }
    let (before, id) = (flight.motion.at, world.get::<NetId>(dragon).map_or(0, |id| id.0));
    flight.motion.carry();
    let crystals = crystals_of(world, layer);
    let mut cx = Ctx { world: &mut *world, layer, dragon, from: before, at: flight.motion.at, crystals, random: &mut *random, dead: false };
    flight::think(&mut cx, flight);
    let dead = cx.dead;
    if let Some(size) = world.get::<Body>(dragon).map(|body| body.size) {
        super::fireball::pushed_by(world, layer, flight.motion.at, size);
    }
    let mut told: Vec<_> = health_told.map(|(left, max)| sapling_net::encode(&sapling_data::mob_health_update(id, left, max))).into_iter().collect();
    if flight.motion.flaps() {
        told.push(sapling_net::encode(&sound("flap", id, head(flight))));
    }
    if !flight.sitting && flight.growls(random) {
        told.push(sapling_net::encode(&sound("mad", id, head(flight))));
    }
    let age = world.resource::<Tick>().0 - kept.born;
    heal(world, layer, dragon, flight, random, age);
    flight.motion.beat(before, flight.sitting);
    flight.trail.record(super::flight::wrap(flight.motion.yaw), flight.motion.at[1]);
    flight.motion.fly();
    flight.level_out();
    if let Some(mut body) = world.get_mut::<Body>(dragon) {
        (body.at, body.motion) = (flight.motion.at, flight.motion.delta);
    }
    flight.motion.slowed = parts::touch(world, layer, dragon, flight, random);
    let target = flight.prey.and_then(|prey| world.get::<NetId>(prey)).map_or(0, |prey| prey.0 as i64);
    told.extend(wire::tell_target(&mut kept.shown, id, target));
    told.extend(wire::tell(&mut kept.shown, id, flight));
    crate::entities::tell_viewers(world, dragon, layer, kept.flight.motion.at.map(|c| c.floor() as i32), told);
    dead
}

fn head(flight: &Flight) -> [f32; 3] {
    let [x, y, z] = flight.motion.at;
    [x, y + HEAD_UP, z]
}

/// Its last flight ended at the portal: its health goes, and its death begins.
pub(super) fn ended(world: &mut World, dragon: Entity) {
    let (Some(id), Some(layer), Some(at)) = (world.get::<NetId>(dragon).map(|id| id.0), world.get::<InLayer>(dragon).map(|l| l.0), world.get::<Body>(dragon).map(|b| b.at)) else { return };
    let event = EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::EnderDragonDeath, data: 0, fire_at_position: None };
    crate::entities::tell_viewers(world, dragon, layer, at.map(|c| c.floor() as i32), vec![sapling_net::encode(&event)]);
}
