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

/// Tells the player and its viewers of one effect at once, before the command's answer (bds-bg2player.txt),
/// stamped with the tick it is told in as BDS's (bds-r1buckets.txt).
pub(super) fn tell_effect(world: &mut World, player: Entity, id: u64, event_id: MobEffectEventId, active: &Active) {
    let tick = world.resource::<crate::components::Tick>().0;
    let told = MobEffect { runtime_entity_id: id, event_id, effect_id: active.effect.id(), amplifier: i32::from(active.amplifier), particles: active.particles, duration: active.left as i32, tick, ambient: active.ambient };
    tell_seen(world, player, sapling_net::encode(&told));
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

/// What a player's effects show of it: the ids of those that show their particles, in order,
/// and whether it is invisible.
pub(super) fn shown(world: &World, player: Entity) -> (Vec<i32>, bool) {
    (shown_of(world.get::<Effects>(player)), super::invisible::is_invisible(world, player))
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

/// Tells the player and its viewers what its effects show, where it is not `before`'s: the flags
/// invisibility gives or takes (invisible.rs), then the effects it shows, in one SetEntityData at
/// the tick-end sync stamped with its input's tick (bds-bg2player.txt, bds-lateeffects.txt).
pub(super) fn show(world: &mut World, player: Entity, id: u64, (before, hidden): &(Vec<i32>, bool)) {
    let (now, hides) = shown(world, player);
    let mut told = sapling_data::visible_effects(id, now.clone());
    if now == *before {
        told.metadata.clear();
    }
    if hides != *hidden {
        told.metadata.splice(0..0, super::invisible::flip(world, player, hides));
    }
    if told.metadata.is_empty() {
        return;
    }
    told.tick = world.get::<crate::movement::Motion>(player).map_or(0, crate::movement::Motion::input_tick);
    tell_at_sync(world, player, sapling_net::encode(&told));
}

/// Sends `packet` to the player and its viewers once this tick's actors are synced, in their order
/// (entities/slots.rs).
pub(super) fn tell_at_sync(world: &mut World, player: Entity, packet: bytes::Bytes) {
    crate::entities::told_after_answer(world, |world| tell_seen(world, player, packet));
}

/// Sends `packet` now to the player and those who see it.
fn tell_seen(world: &mut World, player: Entity, packet: bytes::Bytes) {
    if let Some(mut out) = world.get_mut::<crate::components::Outgoing>(player) {
        out.0.push(packet.clone());
    }
    for (sees, mut out) in world.query::<(&crate::visibility::Sees, &mut crate::components::Outgoing)>().iter_mut(world) {
        if sees.0.contains(&player) {
            out.0.push(packet.clone());
        }
    }
}
