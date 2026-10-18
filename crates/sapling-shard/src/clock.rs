//! The time of day. Each world's counter (`sapling_world::Level::time`) advances a tick at a time
//! while its `doDayLightCycle` rule is on. A client counts on by itself from what it was last
//! told, so it is told at the join, when the time is set or stopped, and now and then to stay in
//! step, in the packets of `sapling_data::world_clocks`.

use acacia_proto::packets::SyncWorldClocks;
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_data::DAYLIGHT_CYCLE;
use sapling_world::Level;

use crate::components::{InLayer, Layer, Outgoing, Stage, Tick};
use crate::dimensions::{self, Realm};
use crate::chat;

/// Ticks between the reminders that keep clients in step (BDS: about 256).
const SYNC_INTERVAL: u64 = 256;

fn wire_time(level: &Level) -> i32 {
    level.time.rem_euclid(i64::from(i32::MAX) + 1) as i32
}

fn runs(level: &Level) -> bool {
    level.rules.on(DAYLIGHT_CYCLE)
}

/// What a joining player is told.
pub(crate) fn registry(level: &Level) -> SyncWorldClocks {
    sapling_data::world_clocks(wire_time(level), !runs(level))
}

fn state(level: &Level) -> SyncWorldClocks {
    sapling_data::clock_state(wire_time(level), !runs(level))
}

/// Advances every world's clock, the other dimensions of an overworld with it, and reminds
/// their players of it every [`SYNC_INTERVAL`] ticks.
pub(crate) fn advance(world: &mut World) {
    for (layer, mut level, realm) in world.query::<(Entity, &mut Layer, Option<&Realm>)>().iter_mut(world) {
        if realm.is_none_or(|realm| realm.0[0] == Some(layer)) {
            let level = level.world.level_mut();
            level.time += i64::from(runs(level));
        }
    }
    dimensions::share_levels(world);
    if world.resource::<Tick>().0 % SYNC_INTERVAL != 0 {
        return;
    }
    let reminders: Vec<(Entity, Bytes)> = world.query::<(Entity, &Layer)>().iter(world).map(|(entity, layer)| (entity, sapling_net::encode(&state(layer.world.level())))).collect();
    for (layer, reminder) in reminders {
        for (_, _, mut out) in world.query::<(&Stage, &InLayer, &mut Outgoing)>().iter_mut(world).filter(|(stage, in_layer, _)| **stage == Stage::InWorld && in_layer.0 == layer) {
            out.0.push(reminder.clone());
        }
    }
}

/// Sets a world's time, that of its overworld for another dimension, and tells its players at once.
pub(crate) fn set(world: &mut World, layer: Entity, time: i64) {
    let home = dimensions::home(world, layer);
    let Some(mut level) = world.get_mut::<Layer>(home) else { return };
    level.world.level_mut().time = time;
    dimensions::share_levels(world);
    tell_layer(world, layer);
}

/// Tells a world's players, and those of its other dimensions, its time and whether it runs.
pub(crate) fn tell_layer(world: &mut World, layer: Entity) {
    let Some(now) = world.get::<Layer>(layer).map(|layer| sapling_net::encode(&state(layer.world.level()))) else { return };
    dimensions::members(world, layer).into_iter().for_each(|member| chat::send_to_layer(world, member, &now));
}

/// Tells a player who came from another world what time it is in this one.
pub(crate) fn tell(world: &mut World, layer: Entity, player: Entity) {
    let Some(now) = world.get::<Layer>(layer).map(|layer| state(layer.world.level())) else { return };
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.push(&now);
    }
}
