//! A mob's entity properties (`sapling_data::entity_properties`), set as its kind's events in the
//! vanilla packs set them: on spawning a grown one rolls its voice (`randomize_sound_variant`, each
//! alike), a farm animal takes its biome's climate and a zombie nautilus a warm shallow ocean's
//! coral; a bred one rolls its voice where its kind's `entity_born` does and takes a parent's
//! climate (`property_inheritance`); a cat rolls again when grown. BDS announced them so
//! (research/captures/bds-properties.txt: 8 pigs, 6 cows, chickens and cats, 8 wolves), and tells
//! a change in a SetEntityData of the properties changed.
// TODO: properties are not saved with a mob, and a mob a world folder kept is shown as its kind
// was recorded; a summoned grown farm animal or cat is never a baby (`entity_spawned`'s 5 and 25%).

use acacia_proto::packets::{AddEntity, SetEntityData};
use acacia_proto::types::{
    EntityProperties, EntityPropertiesIntsItem, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value,
    MetadataDictionaryItemValueDefault as Plain,
};
use bevy_ecs::prelude::*;

use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId};
use crate::entities::{self, Kind};

const SOUND: &str = "minecraft:sound_variant";
const CLIMATE: &str = "minecraft:climate_variant";
const NAUTILUS: &str = "minecraft:zombie_nautilus";
/// Kinds whose grown ones roll their voice as they spawn.
const VOICED: &[&str] = &["minecraft:pig", "minecraft:cow", "minecraft:chicken", "minecraft:cat", "minecraft:wolf"];
/// Kinds whose `entity_born` rolls it (a bred wolf or cat is a tame baby, which does not).
const VOICED_AT_BIRTH: &[&str] = &["minecraft:pig", "minecraft:cow", "minecraft:chicken"];
/// Kinds that take their biome's climate.
const FARM_ANIMALS: &[&str] = &["minecraft:pig", "minecraft:cow", "minecraft:chicken"];
const CLIMATES: [(&str, &str); 2] = [("spawns_warm_variant_farm_animals", "warm"), ("spawns_cold_variant_farm_animals", "cold")];

/// Dice of their own, so the rolls here move no other draw of the shard's.
#[derive(Resource, Default)]
struct Rolls(Dice);

fn dice(world: &mut World) -> Mut<'_, Rolls> {
    world.get_resource_or_insert_with(Rolls::default)
}

/// Its properties set off their kind's defaults, by number.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub(crate) struct Properties(Vec<(u32, i32)>);

impl Properties {
    pub fn get(&self, index: u32) -> Option<i32> {
        self.0.iter().find(|(at, _)| *at == index).map(|(_, value)| *value)
    }

    fn put(&mut self, index: u32, value: i32) {
        match self.0.iter_mut().find(|(at, _)| *at == index) {
            Some((_, old)) => *old = value,
            None => self.0.push((index, value)),
        }
    }

    fn put_named(&mut self, kind: &str, name: &str, value: &str) {
        if let Some((index, value)) = sapling_data::entity_property(kind, name).and_then(|property| Some((property.index, property.value_of(value)?))) {
            self.put(index, value);
        }
    }

    /// Every property of `kind` as AddEntity carries them.
    pub fn shown(&self, kind: &str) -> EntityProperties {
        sapling_data::property_values(kind, &self.0)
    }
}

fn roll_voice(dice: &mut Dice, kind: &str, properties: &mut Properties) {
    if let Some(property) = sapling_data::entity_property(kind, SOUND)
        && let sapling_data::PropertyShape::Enum(values) = &property.shape
    {
        properties.put(property.index, dice.roll(values.len() as u32) as i32);
    }
}

/// What a mob of `kind` coming at `feet` in `layer` is given: as one spawned, or as one `bred`.
pub(super) fn rolled(world: &mut World, layer: Entity, kind: &str, feet: [f32; 3], bred: bool) -> Properties {
    let mut properties = Properties::default();
    if (if bred { VOICED_AT_BIRTH } else { VOICED }).contains(&kind) {
        roll_voice(&mut dice(world).0, kind, &mut properties);
    }
    let [x, y, z] = feet.map(|c| c.floor() as i32);
    let Some(biome) = world.get::<Layer>(layer).filter(|_| !bred).and_then(|layer| layer.world.biome(x, y, z)) else { return properties };
    let has = |tag: &str| sapling_data::biome_has_tag(biome, tag);
    if FARM_ANIMALS.contains(&kind)
        && let Some((_, climate)) = CLIMATES.iter().find(|(tag, _)| has(tag))
    {
        properties.put_named(kind, CLIMATE, climate);
    }
    if kind == NAUTILUS && has("ocean") && has("warm") && !has("deep") {
        properties.put_named(kind, "minecraft:variant", "coral");
    }
    properties
}

/// AddEntity with the mob's properties, where its kind has any.
pub(super) fn dress(added: &mut AddEntity, properties: Option<&Properties>) {
    if sapling_data::entity_properties(&added.entity_type).next().is_some() {
        added.properties = properties.cloned().unwrap_or_default().shown(&added.entity_type);
    }
}

/// Sets the mob's property `name` to `value` (its number) and tells its viewers, with `metadata`
/// BDS sent beside the change; nothing when the property already holds it.
pub(crate) fn set(world: &mut World, entity: Entity, name: &str, value: i32, metadata: Vec<MetadataDictionaryItem>) {
    let Some(kind) = world.get::<Kind>(entity).map(|kind| kind.0.clone()) else { return };
    let Some(property) = sapling_data::entity_property(&kind, name) else { return };
    let index = property.index;
    let held = world.get::<Properties>(entity).and_then(|properties| properties.get(index)).unwrap_or(property.default);
    if held == value {
        return;
    }
    match world.get_mut::<Properties>(entity) {
        Some(mut properties) => properties.put(index, value),
        None => {
            let mut properties = Properties::default();
            properties.put(index, value);
            world.entity_mut(entity).insert(properties);
        }
    }
    let (Some(id), Some(layer), Some(body)) = (world.get::<NetId>(entity).map(|id| id.0), world.get::<InLayer>(entity).map(|layer| layer.0), world.get::<Body>(entity).copied()) else { return };
    let properties = EntityProperties { ints: vec![EntityPropertiesIntsItem { index, value }], floats: Vec::new() };
    let packet = sapling_net::encode(&SetEntityData { runtime_entity_id: id, metadata, properties, tick: 0 });
    entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), vec![packet]);
    super::shown::reshow(world, entity);
}

/// The `AmbientSoundEventName` BDS sent beside a property that switched a mob's sounds.
pub(super) fn ambient_sound(name: &str) -> MetadataDictionaryItem {
    MetadataDictionaryItem { key: Key::AmbientSoundEventName, r#type: Type::String, legacy_type: 4, value: Value::Default(Plain::String(name.into())) }
}

/// The mob `baby` of `kind` was bred by `parents`: it takes one's climate, either alike.
// TODO: which parent BDS's `property_inheritance` takes from was not drilled.
pub(super) fn inherit(world: &mut World, baby: Entity, parents: [Entity; 2], kind: &str) {
    let Some(index) = sapling_data::entity_property(kind, CLIMATE).map(|property| property.index) else { return };
    let parent = parents[dice(world).0.roll(2) as usize];
    let climate = world.get::<Properties>(parent).and_then(|properties| properties.get(index)).unwrap_or(0);
    set(world, baby, CLIMATE, climate, Vec::new());
}

/// The mob of `kind` grew up: a cat rolls its voice again (`ageable_grow_up`).
pub(super) fn grew_up(world: &mut World, entity: Entity, kind: &str) {
    if kind != "minecraft:cat" {
        return;
    }
    let mut rolled = Properties::default();
    roll_voice(&mut dice(world).0, kind, &mut rolled);
    if let Some(&(_, value)) = rolled.0.first() {
        set(world, entity, SOUND, value, Vec::new());
    }
}
