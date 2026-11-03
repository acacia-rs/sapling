//! What a mob's viewers are told of its effects: `VisibleMobEffects`, and for speed and slowness
//! alone a `MobEffect`.

use acacia_proto::packets::{MobEffect, MobEffectEventId};
use bevy_ecs::prelude::*;
use sapling_api::Effect;

use super::{Active, Afflicted};
use crate::body::Body;
use crate::components::{InLayer, NetId};
use crate::entities;

/// Whether BDS tells a mob's viewers of the effect with a `MobEffect`, as it did speed and slowness alone.
pub(super) fn is_told(effect: Effect) -> bool {
    matches!(effect, Effect::Speed | Effect::Slowness)
}

pub(super) fn told(id: u64, event_id: MobEffectEventId, active: &Active) -> bytes::Bytes {
    sapling_net::encode(&MobEffect { runtime_entity_id: id, event_id, effect_id: active.effect.id(), amplifier: i32::from(active.amplifier), particles: active.particles, duration: active.left as i32, tick: 0, ambient: false })
}

/// The mob's `VisibleMobEffects` (those with particles) where `active` shows other effects than
/// `before`, also written into its announcement for viewers coming later; BDS sent it after the
/// `MobEffect` and not for an unshown one (research/captures/bds-lateeffects.txt).
pub(super) fn shown_and_announced(world: &mut World, mob: Entity, id: u64, before: &[Active], active: &[Active]) -> Option<bytes::Bytes> {
    let shown = visible_effects(id, active);
    if shown.metadata == visible_effects(id, before).metadata {
        return None;
    }
    crate::mobs::reannounce_item(world, mob, shown.metadata[0].clone());
    Some(sapling_net::encode(&shown))
}

fn visible_effects(id: u64, active: &[Active]) -> acacia_proto::packets::SetEntityData {
    sapling_data::visible_effects(id, active.iter().filter(|active| active.particles).map(|active| active.effect.id()))
}

/// The `VisibleMobEffects` an afflicted mob's announcement carries, for one rebuilt.
pub(in crate::creatures) fn visible(world: &World, mob: Entity) -> Option<acacia_proto::types::MetadataDictionaryItem> {
    Some(visible_effects(world.get::<NetId>(mob)?.0, &world.get::<Afflicted>(mob)?.0).metadata.remove(0))
}

pub(super) fn tell(world: &mut World, mob: Entity, packets: Vec<bytes::Bytes>) {
    let (Some(layer), Some(at)) = (world.get::<InLayer>(mob).map(|layer| layer.0), world.get::<Body>(mob).map(|body| body.at)) else { return };
    entities::tell_viewers(world, mob, layer, at.map(|c| c.floor() as i32), packets);
}
