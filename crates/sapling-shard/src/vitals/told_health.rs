//! A player's health as told: where it changed since last told, at once, or after the events of what hurt it.
//! BDS tells a player crushed by a landing anvil or stalactite its health after the landing's events, the
//! hurt flash before them (research/captures/bds-v2drip.txt, bds-z2open.txt).

use bevy_ecs::prelude::*;

use super::{MAX, Vitals, boost, effects, tell};
use crate::components::NetId;

/// The players whose health changed while a hold was open, told by [`tell_held`].
#[derive(Resource, Default)]
struct Held(Option<Vec<Entity>>);

/// Runs `hurts` with the health changes it makes held back; what it returns, and the players held.
pub(crate) fn held<R>(world: &mut World, hurts: impl FnOnce(&mut World) -> R) -> (R, Vec<Entity>) {
    let opens = world.get_resource_or_insert_with(Held::default).0.is_none();
    if opens {
        world.resource_mut::<Held>().0 = Some(Vec::new());
    }
    let done = hurts(world);
    let players = if opens { world.resource_mut::<Held>().0.take().unwrap_or_default() } else { Vec::new() };
    (done, players)
}

/// Tells the health of the players [`held`] held back.
pub(crate) fn tell_held(world: &mut World, players: Vec<Entity>) {
    for player in players {
        tell_changed(world, player);
    }
}

/// Tells the player's health where it changed since it was last told, unless a hold keeps it back.
pub(super) fn tell_changed(world: &mut World, player: Entity) {
    let Some((left, most, told)) = world.get::<Vitals>(player).map(|vitals| (vitals.health.left(), vitals.health.max(), vitals.told)) else { return };
    if told == left {
        return;
    }
    if let Some(held) = world.get_resource_mut::<Held>().and_then(|held| held.into_inner().0.as_mut()) {
        if !held.contains(&player) {
            held.push(player);
        }
        return;
    }
    let Some(id) = world.get::<NetId>(player).map(|id| id.0) else { return };
    if let Some(mut vitals) = world.get_mut::<Vitals>(player) {
        vitals.told = left;
    }
    // BDS kept the boost's modifier on every later health it told (bds-u1fly.txt).
    let boosted = effects::level(world, player, sapling_api::Effect::HealthBoost);
    let told = if boosted.is_some() { boost::boosted_health(id, left, MAX, boosted) } else { sapling_data::mob_health_update(id, left, most) };
    tell(world, player, &told);
}
