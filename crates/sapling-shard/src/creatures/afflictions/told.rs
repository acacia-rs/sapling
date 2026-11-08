//! What a mob's viewers are told of its effects: `VisibleMobEffects`, the fire immune flag, and for
//! speed and slowness alone a `MobEffect`.

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

/// A mob's omen taking hold (`vitals::effects::omen_sound`), heard 0.85 of its shown height up (a
/// zombie's 0.9, a villager's at its feet) and 0.9 of its width ahead along its head's yaw: bds-bd2mouth.txt's
/// cow, calf, pig, chicken, zombie and villager, bds-bd2event.txt's sheep.
pub(super) fn omen_heard(world: &World, mob: Entity, id: u64, sound: &str) -> Option<bytes::Bytes> {
    let (kind, body) = (world.get::<entities::Kind>(mob)?.0.as_str(), world.get::<Body>(mob)?);
    let baby = crate::creatures::shown::is_baby(world, mob);
    let [width, height] = shown_size(world, mob, kind, body);
    let legs = || world.get::<crate::creatures::legs::Legs>(mob).map(|legs| legs.yaw);
    let yaw = crate::creatures::gaze::head_yaw(world, mob).or_else(legs).unwrap_or(0.0).to_radians();
    let (up, ahead) = match kind {
        "minecraft:villager_v2" => (0.0, 0.0),
        "minecraft:zombie" => (0.9 * height, 0.9 * width),
        _ => (0.85 * height, 0.9 * width),
    };
    let [x, y, z] = body.at;
    let position = acacia_proto::types::Vec3f { x: x - yaw.sin() * ahead, y: y + up, z: z + yaw.cos() * ahead };
    let heard = acacia_proto::packets::LevelSoundEvent { sound_id: sound.into(), position, extra_data: -1, entity_type: kind.into(), is_baby_mob: baby, is_global: false, entity_unique_id: id as i64, fire_at_position: None };
    Some(sapling_net::encode(&heard))
}

/// The mob's box as shown: a baby's scaled.
pub(super) fn shown_size(world: &World, mob: Entity, kind: &str, body: &Body) -> [f32; 2] {
    let baby = crate::creatures::shown::is_baby(world, mob);
    sapling_data::mob_look(kind, baby).filter(|_| baby).map_or(body.size, |(size, scale)| size.map(|side| side * scale))
}

pub(super) fn told(id: u64, event_id: MobEffectEventId, active: &Active) -> bytes::Bytes {
    sapling_net::encode(&MobEffect { runtime_entity_id: id, event_id, effect_id: active.effect.id(), amplifier: i32::from(active.amplifier), particles: active.particles, duration: active.left as i32, tick: 0, ambient: false })
}

/// The mob's `VisibleMobEffects` (those with particles) where `active` shows other effects than
/// `before`, also written into its announcement for viewers coming later; BDS sent it after the
/// `MobEffect` and not for an unshown one (research/captures/bds-lateeffects.txt).
/// Fire resistance coming or going flips the `FIRE_IMMUNE` flag in the same data, first; water
/// breathing shows no flag (research/captures/bds-t2flags.txt); invisibility: invisible.rs.
pub(super) fn shown_and_announced(world: &mut World, mob: Entity, id: u64, before: &[Active], active: &[Active]) -> Option<bytes::Bytes> {
    let resists = |list: &[Active]| list.iter().any(|active| active.effect == Effect::FireResistance);
    let flags = (resists(before) != resists(active)).then(|| fire_immune(world, mob, id, resists(active))).flatten();
    let hides = super::invisible::is_invisible(active);
    let flags = (super::invisible::is_invisible(before) != hides).then(|| super::invisible::flip(world, mob, id, hides)).flatten().or(flags);
    let mut shown = visible_effects(id, active);
    let visible = shown.metadata != visible_effects(id, before).metadata;
    if visible {
        crate::mobs::reannounce_item(world, mob, shown.metadata[0].clone());
    } else {
        shown.metadata.clear();
    }
    shown.metadata.splice(0..0, flags);
    (!shown.metadata.is_empty()).then(|| sapling_net::encode(&shown))
}

fn fire_immune(world: &mut World, mob: Entity, id: u64, on: bool) -> Option<acacia_proto::types::MetadataDictionaryItem> {
    let kind = world.get::<entities::Kind>(mob)?.0.clone();
    if sapling_data::mob_traits(&kind).fire_immune {
        return None;
    }
    let flags = entities::mob_flags(world, mob, &kind, acacia_proto::types::MetadataFlags1::FIRE_IMMUNE, on);
    Some(sapling_data::mob_flags_set(id, flags).metadata.remove(0))
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

/// [`tell`], held to BDS's actor-data sync at the tick's end: `VisibleMobEffects` and the omen sound came
/// after `/effect`'s answer, mobs in the sync's order (bds-bd2event.txt; entities/slots.rs).
pub(super) fn tell_at_sync(world: &mut World, mob: Entity, packets: Vec<bytes::Bytes>) {
    if !packets.is_empty() {
        entities::told_after_answer(world, |world| tell(world, mob, packets));
    }
}
