//! A horse-family mob's open mouth: bit 128 of its actor data 16 (`MinecartDisplayBlock` on the
//! wire), Java's `AbstractHorse.openMouth`. BDS's opened at every hurt, `/damage` and `/kill`
//! alike, and shut it 29 ticks after; a llama has none (research/captures/bds-firstdata.txt,
//! bds-mountdeath.txt). A saddled one's death also shows it unsaddled, in the same packet.
// TODO: BDS's horse also showed 32 (rearing) in this data, idle; not ported.

use acacia_proto::packets::SetEntityData;
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use bevy_ecs::prelude::*;

use super::mounts::{TAMED_SADDLED, saddle_flags};
use crate::body::Body;
use crate::components::{InLayer, NetId, Tick};
use crate::creatures::{Health, is_dying};
use crate::entities::{self, Kind};

const OPEN: i64 = 128;
const OPEN_FOR: u64 = 29;

/// The health last seen, and the tick the open mouth shuts.
#[derive(Component)]
pub(super) struct Mouth {
    seen: f32,
    shuts: Option<u64>,
}

fn mouth_item(open: bool) -> MetadataDictionaryItem {
    MetadataDictionaryItem { key: Key::MinecartDisplayBlock, r#type: Type::Long, legacy_type: 7, value: Value::Default(Plain::Long(if open { OPEN } else { 0 })) }
}

fn data(id: u64, metadata: Vec<MetadataDictionaryItem>) -> bytes::Bytes {
    sapling_net::encode(&SetEntityData { runtime_entity_id: id, metadata, properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 })
}

/// Opens the mouth of each living horse-family mob hurt since the last tick, and shuts those due.
pub(crate) fn open_mouths(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let horses: Vec<(Entity, Entity, u64, f32, [f32; 3])> = world
        .query::<(Entity, &Kind, &Health, &NetId, &InLayer, &Body)>()
        .iter(world)
        .filter(|(_, kind, ..)| TAMED_SADDLED.contains(&kind.0.as_str()))
        .map(|(entity, _, health, id, layer, body)| (entity, layer.0, id.0, health.left(), body.at))
        .collect();
    for (entity, layer, id, left, at) in horses {
        if is_dying(world, entity) {
            continue;
        }
        let Some(mut mouth) = world.get_mut::<Mouth>(entity) else {
            world.entity_mut(entity).insert(Mouth { seen: left, shuts: None });
            continue;
        };
        let hurt = left < mouth.seen;
        mouth.seen = left;
        let was_open = mouth.shuts.is_some();
        if hurt {
            mouth.shuts = Some(now + OPEN_FOR);
        } else if mouth.shuts.is_some_and(|shuts| shuts <= now) {
            mouth.shuts = None;
        }
        let open = mouth.shuts.is_some();
        if open != was_open {
            entities::tell_viewers(world, entity, layer, at.map(|c| c.floor() as i32), vec![data(id, vec![mouth_item(open)])]);
        }
    }
}

/// At a horse-family mob's death: its mouth opens, and a saddle it let go of leaves its flags
/// (bds-mountdeath.txt), told after the tick's loot is announced as BDS did.
pub(crate) fn fall(world: &mut World, mob: Entity, layer: Entity, kind: &str, body: &Body, unsaddled: bool) {
    let Some(id) = world.get::<NetId>(mob).map(|id| id.0).filter(|_| TAMED_SADDLED.contains(&kind)) else { return };
    let mut metadata = Vec::new();
    if unsaddled {
        let word = entities::mob_flags(world, mob, kind, saddle_flags(kind), false);
        metadata.push(MetadataDictionaryItem { key: Key::Flags, r#type: Type::Long, legacy_type: 7, value: Value::Flags(word) });
    }
    let was_open = world.get::<Mouth>(mob).is_some_and(|mouth| mouth.shuts.is_some());
    world.entity_mut(mob).insert(Mouth { seen: 0.0, shuts: Some(u64::MAX) });
    if !was_open {
        metadata.push(mouth_item(true));
    }
    if !metadata.is_empty() {
        entities::after_announcing_of(world, mob, layer, body.at.map(|c| c.floor() as i32), vec![data(id, metadata)]);
    }
}
