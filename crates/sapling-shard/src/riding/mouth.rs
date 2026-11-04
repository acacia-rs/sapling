//! A horse-family mob's actor data 16 (`MinecartDisplayBlock` on the wire, BDS's `HorseFlags`):
//! bit 128 its open mouth, Java's `AbstractHorse.openMouth`, and bit 32 its grazing (grazing.rs).
//! BDS's opened the mouth at every hurt, `/damage` and `/kill` alike, and shut it 29 ticks after; a
//! llama has none (research/captures/bds-firstdata.txt, bds-mountdeath.txt). A saddled one's death
//! also shows it unsaddled, in the same packet.

use acacia_proto::packets::SetEntityData;
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use bevy_ecs::prelude::*;

use super::grazing::{EATING, Grazing, graze, heal};
use super::mounts::{TAMED_SADDLED, saddle_flags};
use crate::body::Body;
use crate::components::{InLayer, NetId, Tick};
use crate::creatures::{Health, is_dying};
use crate::entities::{self, Kind};

const OPEN: i64 = 128;
const OPEN_FOR: u64 = 29;

/// The health last seen, the tick the open mouth shuts, and the data last told.
#[derive(Component)]
pub(super) struct Mouth {
    seen: f32,
    shuts: Option<u64>,
    told: i64,
}

fn flags_item(flags: i64) -> MetadataDictionaryItem {
    MetadataDictionaryItem { key: Key::MinecartDisplayBlock, r#type: Type::Long, legacy_type: 7, value: Value::Default(Plain::Long(flags)) }
}

fn data(id: u64, metadata: Vec<MetadataDictionaryItem>) -> bytes::Bytes {
    sapling_net::encode(&SetEntityData { runtime_entity_id: id, metadata, properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 })
}

/// Opens the mouth of each living horse-family mob hurt since the last tick, shuts those due,
/// lets each graze, and tells what changed.
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
        let Some(mouth) = world.get::<Mouth>(entity) else {
            world.entity_mut(entity).insert((Mouth { seen: left, shuts: None, told: 0 }, Grazing::default()));
            continue;
        };
        let (hurt, mut shuts, told) = (left < mouth.seen, mouth.shuts, mouth.told);
        if hurt {
            shuts = Some(now + OPEN_FOR);
        } else if shuts.is_some_and(|shuts| shuts <= now) {
            shuts = None;
        }
        heal(world, entity);
        let (eats, stirred) = graze(world, entity, layer, at);
        let flags = if shuts.is_some() { OPEN } else { 0 } | if eats { EATING } else { 0 };
        world.entity_mut(entity).insert(Mouth { seen: left, shuts, told: flags });
        if flags != told || stirred {
            entities::tell_viewers(world, entity, layer, at.map(|c| c.floor() as i32), vec![data(id, vec![flags_item(flags)])]);
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
    let told = world.get::<Mouth>(mob).map_or(0, |mouth| mouth.told);
    world.entity_mut(mob).insert(Mouth { seen: 0.0, shuts: Some(u64::MAX), told: told | OPEN });
    if told & OPEN == 0 {
        metadata.push(flags_item(told | OPEN));
    }
    if !metadata.is_empty() {
        entities::after_announcing_of(world, mob, layer, body.at.map(|c| c.floor() as i32), vec![data(id, metadata)]);
    }
}
