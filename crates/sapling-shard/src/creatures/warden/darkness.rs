//! The darkness a warden casts: from the tick it comes, and every 121 ticks after, darkness for
//! 260 ticks to each survival or adventure player within 20 of it (its pack's `minecraft:mob_effect`;
//! research/captures/bds-warden-findings.md "Darkness").

use bevy_ecs::prelude::*;
use sapling_api::Effect;

use super::Warden;
use crate::body::Body;
use crate::components::{InLayer, Position, Tick, in_world};
use crate::vitals;

const REACH: f32 = 20.0;
const LASTS: u32 = 260;
const EVERY: u64 = 121;

/// When a warden next casts it.
#[derive(Component, Default)]
pub(super) struct Darkening(u64);

pub(super) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let due: Vec<(Entity, Entity, [f32; 3])> = world
        .query_filtered::<(Entity, &InLayer, &Body, Option<&Darkening>), With<Warden>>()
        .iter(world)
        .filter(|(.., darkening)| darkening.is_none_or(|darkening| now >= darkening.0))
        .map(|(warden, layer, body, _)| (warden, layer.0, body.at))
        .collect();
    for (warden, layer, at) in due {
        world.entity_mut(warden).insert(Darkening(now + EVERY));
        let near = |feet: [f32; 3]| (0..3).map(|axis| (feet[axis] - at[axis]).powi(2)).sum::<f32>() <= REACH * REACH;
        let players: Vec<Entity> = in_world(world)
            .into_iter()
            .filter(|player| world.get::<InLayer>(*player).is_some_and(|on| on.0 == layer) && vitals::is_mortal(world, *player))
            .filter(|player| world.get::<Position>(*player).is_some_and(|position| near(position.feet)))
            .collect();
        for player in players {
            vitals::give_effect(world, player, Effect::Darkness, LASTS, 0, false);
        }
    }
}
