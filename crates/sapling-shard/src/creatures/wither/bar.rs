//! A wither's boss bar, shown every second to players within [`BAR_REACH`], and its healing with no
//! player within [`LONELY`].

use acacia_proto::packets::BossEventColor;
use bevy_ecs::prelude::*;

use super::super::health::Health;
use super::Wither;
use crate::body::Body;
use crate::components::{InLayer, NetId, Position, Tick, in_world};

const BAR_REACH: f32 = 55.0;
const LONELY: f32 = 50.0;
pub(super) const SECOND: u64 = 20;
const TITLE: &str = "entity.wither.name";

/// A second's healing with no player near.
pub(super) fn heal_alone(world: &mut World, wither: Entity, layer: Entity, body: &Body) {
    if world.resource::<Tick>().0 % SECOND != 0 {
        return;
    }
    let near = |feet: [f32; 3]| (0..3).all(|axis| (feet[axis] - body.at[axis]).abs() <= LONELY);
    let lonely = !in_world(world).into_iter().any(|player| world.get::<InLayer>(player).is_some_and(|on| on.0 == layer) && world.get::<Position>(player).is_some_and(|at| near(at.feet)));
    if let Some(mut health) = world.get_mut::<Health>(wither).filter(|_| lonely) {
        health.mend(1.0);
    }
}

/// Each wither's bar, every second.
pub(super) fn show(world: &mut World) {
    if world.resource::<Tick>().0 % SECOND != 0 {
        return;
    }
    let bars: Vec<(Entity, i64, f32, [f32; 3])> = world.query_filtered::<(&InLayer, &NetId, &Body, &Health), With<Wither>>().iter(world).map(|(layer, id, body, health)| (layer.0, id.0 as i64, health.left() / health.max(), body.at)).collect();
    for (layer, id, progress, at) in bars {
        crate::dragon::show_boss_bar(world, layer, id, TITLE, BossEventColor::RebeccaPurple, progress, at, BAR_REACH);
    }
}
