//! Plugin-set name and score tags, and floating text: an entity with nothing but a name. A label
//! is told to who sees its entity now (SetEntityData) and to who comes to see it later, after its
//! announcement (visibility.rs, mobs.rs).

use acacia_proto::packets::{AddEntity, SetEntityData};
use acacia_proto::types::{
    EntityProperties, MetadataDictionaryItem as Item, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value,
    MetadataDictionaryItemValueDefault as Plain, MetadataFlags1 as Flags,
};
use acacia_proto::RawPacket;
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::{EntityId, Label};
use sapling_world::ChunkPos;

use crate::components::{Identity, Layer, NetId, Outgoing};
use crate::mobs::{self, SeesMobs};
use crate::visibility::Sees;

/// What plugins put over an entity's head; `None` leaves the game's own.
#[derive(Component, Default)]
pub(crate) struct Labels {
    name: Option<String>,
    score: Option<String>,
}

/// An entity that is only floating text.
#[derive(Component)]
pub(crate) struct FloatingText;

fn string(key: Key, text: &str) -> Item {
    Item { key, r#type: Type::String, legacy_type: 4, value: Value::Default(Plain::String(text.to_owned())) }
}

fn byte(key: Key, value: i8) -> Item {
    Item { key, r#type: Type::Byte, legacy_type: 0, value: Value::Default(Plain::Byte(value)) }
}

fn set_data(id: u64, metadata: Vec<Item>) -> Bytes {
    sapling_net::encode(&SetEntityData { runtime_entity_id: id, metadata, properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 })
}

/// The name the game itself shows over `entity`: a player's own, or the one its announcement gave.
fn own_name(world: &World, entity: Entity) -> String {
    if let Some(identity) = world.get::<Identity>(entity) {
        return identity.0.name.clone();
    }
    let first = mobs::announced(world, entity).and_then(|shown| shown.into_iter().next());
    let added = first.and_then(|first| RawPacket::parse(first).ok()?.decode::<AddEntity>().ok());
    let name = added.and_then(|added| added.metadata.into_iter().find(|item| item.key == Key::Nametag));
    match name.map(|item| item.value) {
        Some(Value::Default(Plain::String(name))) => name,
        _ => String::new(),
    }
}

/// What tells a viewer the labels of the entity `id`; `None` where plugins set none.
pub(crate) fn packet(labels: Option<&Labels>, id: u64) -> Option<Bytes> {
    let labels = labels?;
    let mut metadata = Vec::new();
    if let Some(name) = &labels.name {
        metadata.extend([string(Key::Nametag, name), byte(Key::AlwaysShowNametag, 1)]);
    }
    if let Some(score) = &labels.score {
        metadata.push(string(Key::ScoreTag, score));
    }
    (!metadata.is_empty()).then(|| set_data(id, metadata))
}

/// Sets one label of `entity` (`None`: the game's own back) and tells who sees it.
pub(crate) fn set(world: &mut World, entity: Entity, label: Label, text: Option<&str>) {
    let Some(id) = crate::ui::net_id(world, entity) else { return };
    let mut labels = world.get_mut::<Labels>(entity);
    if labels.is_none() {
        world.entity_mut(entity).insert(Labels::default());
        labels = world.get_mut::<Labels>(entity);
    }
    let mut labels = labels.expect("just inserted");
    let slot = match label {
        Label::NameTag => &mut labels.name,
        Label::ScoreTag => &mut labels.score,
    };
    *slot = text.map(str::to_owned);
    let player = world.get::<Identity>(entity).is_some();
    let told = match (label, text) {
        (Label::NameTag, Some(name)) => vec![string(Key::Nametag, name), byte(Key::AlwaysShowNametag, 1)],
        (Label::NameTag, None) if player => vec![string(Key::Nametag, &own_name(world, entity))],
        (Label::NameTag, None) => vec![string(Key::Nametag, &own_name(world, entity)), byte(Key::AlwaysShowNametag, 0)],
        (Label::ScoreTag, text) => vec![string(Key::ScoreTag, text.unwrap_or(""))],
    };
    to_seers(world, entity, set_data(id, told));
}

/// Hands `packet` to every player whose client has `entity`.
fn to_seers(world: &mut World, entity: Entity, packet: Bytes) {
    let mut viewers = world.query::<(Option<&Sees>, Option<&SeesMobs>, &mut Outgoing)>();
    for (sees, sees_mobs, mut out) in viewers.iter_mut(world) {
        if sees.is_some_and(|sees| sees.0.contains(&entity)) || sees_mobs.is_some_and(|sees| sees.has(entity)) {
            out.0.push(packet.clone());
        }
    }
}

/// The floating text whose id is `id`.
pub(crate) fn floating(world: &World, id: EntityId) -> Option<Entity> {
    world.iter_entities().find(|entity| entity.contains::<FloatingText>() && entity.get::<NetId>().is_some_and(|net| net.0 == id.0)).map(|entity| entity.id())
}

/// The air id a client of `layer` knows.
pub(crate) fn air(world: &World, layer: Entity) -> Option<u32> {
    let registry = world.get::<Layer>(layer)?.world.registry();
    Some(registry.network_hash(registry.air_id()))
}

/// dragonfly's `entity/text.go`: a falling block of air with no box and no gravity, nothing to
/// see but its name, always shown; with an empty name, nothing at all (extra_bars.rs).
pub(crate) fn floating_announcement(id: u64, at: [f32; 3], air: u32, text: &str) -> Vec<Bytes> {
    let mut added = sapling_data::add_falling_block(id, at, [0.0; 3], air);
    let named = if text.is_empty() { 0 } else { Flags::CAN_SHOW_NAMETAG.0 | Flags::ALWAYS_SHOW_NAMETAG.0 };
    let shown = Flags(Flags::NO_AI.0 | named);
    for item in &mut added.metadata {
        match (&item.key, &mut item.value) {
            (Key::Flags, Value::Flags(flags)) => *flags = shown,
            (Key::Nametag, value) => *value = Value::Default(Plain::String(text.to_owned())),
            (Key::BoundingboxWidth | Key::BoundingboxHeight, value) => *value = Value::Default(Plain::Float(0.0)),
            _ => {}
        }
    }
    if !added.metadata.iter().any(|item| item.key == Key::Nametag) {
        added.metadata.push(string(Key::Nametag, text));
    }
    added.metadata.push(byte(Key::AlwaysShowNametag, i8::from(!text.is_empty())));
    vec![sapling_net::encode(&added)]
}

pub(crate) fn add_floating(world: &mut World, layer: Entity, at: [f32; 3], text: &str) -> Option<EntityId> {
    if !world.get::<Layer>(layer)?.world.is_loaded(ChunkPos::of_block(at[0].floor() as i32, at[2].floor() as i32)) {
        return None;
    }
    let air = air(world, layer)?;
    let entity = mobs::spawn_shown(world, layer, at, |id| Some(floating_announcement(id, at, air, text)))?;
    world.entity_mut(entity).insert(FloatingText);
    crate::ui::net_id(world, entity).map(EntityId)
}

pub(crate) fn set_floating(world: &mut World, id: EntityId, text: &str) -> bool {
    let Some(entity) = floating(world, id) else { return false };
    let Some(mut shown) = mobs::announced(world, entity) else { return false };
    // Who comes later is told the new text in the announcement itself.
    if let Some(mut added) = shown.first().and_then(|first| RawPacket::parse(first.clone()).ok()?.decode::<AddEntity>().ok()) {
        for item in added.metadata.iter_mut().filter(|item| item.key == Key::Nametag) {
            item.value = Value::Default(Plain::String(text.to_owned()));
        }
        shown[0] = sapling_net::encode(&added);
        mobs::reannounce(world, entity, shown);
    }
    to_seers(world, entity, set_data(id.0, vec![string(Key::Nametag, text)]));
    true
}
