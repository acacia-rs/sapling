//! The time of day. Each world's counter (`sapling_world::Level::time`) advances a tick at a time
//! while its `doDayLightCycle` rule is on. A client counts on by itself from what it was last
//! told, so it is told at the join, when the time is set or stopped, and now and then to stay in
//! step, in the packets of `sapling_data::world_clocks`.

use acacia_proto::packets::SyncWorldClocks;
use bevy_ecs::prelude::*;
use sapling_data::DAYLIGHT_CYCLE;
use sapling_world::Level;

use crate::chat;
use crate::components::{InLayer, Layer, Outgoing, Stage, Tick};

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

/// Advances every world's clock and reminds its players of it every [`SYNC_INTERVAL`] ticks.
pub(crate) fn advance(
    tick: Res<Tick>,
    mut layers: Query<(Entity, &mut Layer)>,
    mut players: Query<(&Stage, &InLayer, &mut Outgoing)>,
) {
    for (entity, mut layer) in &mut layers {
        let level = layer.world.level_mut();
        level.time += i64::from(runs(level));
        if tick.0 % SYNC_INTERVAL != 0 {
            continue;
        }
        let reminder = sapling_net::encode(&state(level));
        for (_, _, mut out) in players
            .iter_mut()
            .filter(|(stage, in_layer, _)| **stage == Stage::InWorld && in_layer.0 == entity)
        {
            out.0.push(reminder.clone());
        }
    }
}

/// Sets a world's time and tells its players at once.
pub(crate) fn set(world: &mut World, layer: Entity, time: i64) {
    let Some(mut level) = world.get_mut::<Layer>(layer) else {
        return;
    };
    level.world.level_mut().time = time;
    tell_layer(world, layer);
}

/// Tells a world's players its time and whether it runs.
pub(crate) fn tell_layer(world: &mut World, layer: Entity) {
    let Some(now) = world
        .get::<Layer>(layer)
        .map(|layer| sapling_net::encode(&state(layer.world.level())))
    else {
        return;
    };
    chat::send_to_layer(world, layer, &now);
}

/// Tells a player who came from another world what time it is in this one.
pub(crate) fn tell(world: &mut World, layer: Entity, player: Entity) {
    let Some(now) = world
        .get::<Layer>(layer)
        .map(|layer| state(layer.world.level()))
    else {
        return;
    };
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.push(&now);
    }
}
