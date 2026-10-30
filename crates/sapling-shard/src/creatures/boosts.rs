//! Health boost on a mob, as on BDS's cows (research/captures/bds-hits2.txt, bds-boosts.txt): its
//! most health grows as a player's does (`vitals::boost`), and every health its viewers are told
//! carries the boost.

use acacia_proto::packets::UpdateAttributes;
use bevy_ecs::prelude::*;
use sapling_api::Effect;

use super::afflictions::level;
use super::health::Health;
use crate::body::Body;
use crate::components::{InLayer, NetId};
use crate::entities;
use crate::vitals::boost::{boosted_health, boosted_max};

/// The most health a boosted mob has of its own.
#[derive(Component)]
pub(super) struct Unboosted(f32);

/// Sets the mob's most health under the health boost on it now, and tells its viewers.
pub(super) fn boost(world: &mut World, mob: Entity) {
    let boosted = level(world, mob, Effect::HealthBoost);
    let Some(base) = world.get::<Unboosted>(mob).map(|own| own.0).or_else(|| world.get::<Health>(mob).map(Health::max)) else { return };
    match boosted {
        Some(_) => world.entity_mut(mob).insert(Unboosted(base)),
        None => world.entity_mut(mob).remove::<Unboosted>(),
    };
    let (Some(id), Some(layer), Some(at)) = (world.get::<NetId>(mob).map(|id| id.0), world.get::<InLayer>(mob).map(|layer| layer.0), world.get::<Body>(mob).map(|body| body.at)) else { return };
    let Some(mut health) = world.get_mut::<Health>(mob) else { return };
    health.grow(boosted_max(base, boosted));
    health.told = health.left();
    let told = boosted_health(id, health.left(), base, boosted);
    entities::tell_viewers(world, mob, layer, at.map(|c| c.floor() as i32), vec![sapling_net::encode(&told)]);
}

/// What tells the mob's `health` now, with its boost if it has one.
pub(super) fn told(world: &World, mob: Entity, id: u64, health: &Health) -> UpdateAttributes {
    match world.get::<Unboosted>(mob) {
        Some(own) => boosted_health(id, health.left(), own.0, level(world, mob, Effect::HealthBoost)),
        None => sapling_data::mob_health_update(id, health.left(), health.max()),
    }
}
