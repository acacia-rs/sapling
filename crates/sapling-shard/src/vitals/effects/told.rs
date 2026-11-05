//! What a player and its viewers are told of its effects, and the health they put on top.

use acacia_proto::packets::{MobEffect, MobEffectEventId};
use bevy_ecs::prelude::*;
use sapling_api::Effect;

use super::super::{Vitals, settle, tell};
use super::{Active, Effects, level, tell_pace};
use crate::components::NetId;
use crate::creatures::Cause;

/// Tells a player in the world the effects it came back with, once.
pub(super) fn greet(world: &mut World, player: Entity) {
    let (Some(id), Some(mut effects)) = (world.get::<NetId>(player).map(|id| id.0), world.get_mut::<Effects>(player).filter(|effects| effects.2)) else { return };
    effects.2 = false;
    for active in effects.0.clone() {
        tell_effect(world, player, id, MobEffectEventId::Add, &active);
        match active.effect {
            Effect::Speed | Effect::Slowness => tell_pace(world, player, id),
            Effect::HealthBoost => boost(world, player, id),
            _ => {}
        }
    }
}

/// Tells the player of one effect, stamped with the tick it is told in as BDS's (bds-r1buckets.txt).
pub(super) fn tell_effect(world: &mut World, player: Entity, id: u64, event_id: MobEffectEventId, active: &Active) {
    let tick = world.resource::<crate::components::Tick>().0;
    let told = MobEffect { runtime_entity_id: id, event_id, effect_id: active.effect.id(), amplifier: i32::from(active.amplifier), particles: active.particles, duration: active.left as i32, tick, ambient: active.ambient };
    tell(world, player, &told);
}

/// Puts `amount` of health on top of a player's own, told at once.
pub(super) fn pad(world: &mut World, player: Entity, amount: f32) {
    if let Some(mut vitals) = world.get_mut::<Vitals>(player) {
        vitals.health.pad(amount);
    }
    settle(world, player, Cause::Generic);
}

/// Sets the most health a player has under the health boost on it now, and tells it.
pub(super) fn boost(world: &mut World, player: Entity, id: u64) {
    let boosted = level(world, player, Effect::HealthBoost);
    let Some(mut vitals) = world.get_mut::<Vitals>(player) else { return };
    vitals.health.grow(super::super::boost::boosted_max(super::super::MAX, boosted));
    let told = super::super::boost::boosted_health(id, vitals.health.left(), super::super::MAX, boosted);
    tell(world, player, &told);
}

/// The ids of the effects on a player that show their particles, in order.
pub(super) fn shown(world: &World, player: Entity) -> Vec<i32> {
    shown_of(world.get::<Effects>(player))
}

fn shown_of(effects: Option<&Effects>) -> Vec<i32> {
    let mut ids: Vec<i32> = effects.map_or_else(Vec::new, |effects| effects.0.iter().filter(|active| active.particles).map(|active| active.effect.id()).collect());
    ids.sort_unstable();
    ids
}

/// The `VisibleMobEffects` a player's AddPlayer carries: BDS announced one under jump boost
/// with 16 (research/captures/bds-lateeffects.txt).
pub(crate) fn announced(id: u64, effects: Option<&Effects>) -> acacia_proto::types::MetadataDictionaryItem {
    sapling_data::visible_effects(id, shown_of(effects)).metadata.remove(0)
}

/// Tells the player and its viewers the effects it shows, where they are not `before`'s.
pub(super) fn show(world: &mut World, player: Entity, id: u64, before: &[i32]) {
    let now = shown(world, player);
    if now != before {
        tell(world, player, &sapling_data::visible_effects(id, now));
    }
}
