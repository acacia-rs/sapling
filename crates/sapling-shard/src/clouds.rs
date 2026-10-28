//! Area effect clouds (`minecraft:area_effect_cloud`), as BDS's `AreaEffectCloud::normalTick`
//! 1429b01a0 runs them (docs/research/items.md, "Clouds"): the dragon's breath (dragon/cloud.rs)
//! and a lingering potion's (shots/clouds.rs). Its age counts from the tick after it comes; it is
//! gone past its life or under half a block wide. Every fifth tick of its age each player and mob
//! whose feet are within its radius, and that it spared long enough, is given its potion: a quarter
//! of each effect's time, an instant one at half strength. Each one given changes its radius.

use std::collections::HashMap;

use acacia_proto::packets::{AddEntity, SetEntityData};
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_data::AddEntityOptions;

use crate::body::Body;
use crate::components::{InLayer, Position, Tick, in_world};
use crate::{entities, mobs};

pub(crate) const KIND: &str = "minecraft:area_effect_cloud";
pub(crate) const SIZE: [f32; 2] = [0.6, 1.8];
const FLAGS: u64 = 0x6_0000_0000_0000;
const PARTICLE: i32 = 49;
const SHRINK_ON_PICKUP: f32 = -0.5;
const PULSE: u64 = 5;
const SMALLEST: f32 = 0.5;
/// Of an effect's time, and of an instant one's strength, what one pulse gives.
const TIME_SHARE: f32 = 0.25;
const INSTANT_SHARE: f32 = 0.5;
/// Mobs no cloud gives to.
const UNAFFECTED: &[&str] = &["minecraft:ender_dragon"];

/// What a cloud is: its potion and colour, its radius and how that changes a tick of its age and
/// for each one given, its life, and the ticks one given is spared.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Spec {
    pub potion: i16,
    pub colour: i32,
    pub radius: f32,
    pub per_tick: f32,
    pub on_use: f32,
    pub ticks: u64,
    pub again: u64,
}

#[derive(Component)]
struct Cloud {
    spec: Spec,
    /// Who a player it kills was slain by.
    slayer: String,
    born: Option<u64>,
    used: f32,
    spared: HashMap<Entity, u64>,
}

pub(crate) fn item(key: Key, value: Plain) -> MetadataDictionaryItem {
    let (r#type, legacy_type) = match value {
        Plain::Byte(_) => (Type::Byte, 0),
        Plain::Short(_) => (Type::Short, 1),
        Plain::Int(_) => (Type::Int, 2),
        Plain::Float(_) => (Type::Float, 3),
        Plain::Long(_) => (Type::Long, 7),
        _ => (Type::String, 4),
    };
    MetadataDictionaryItem { key, r#type, legacy_type, value: Value::Default(value) }
}

/// Sets `items` over `metadata`'s, appending those it lacks, and its flags to `flags`.
pub(crate) fn with(metadata: &mut Vec<MetadataDictionaryItem>, flags: u64, items: Vec<MetadataDictionaryItem>) {
    for added in items {
        metadata.retain(|item| item.key != added.key);
        metadata.push(added);
    }
    for item in metadata.iter_mut().filter(|item| item.key == Key::Flags) {
        item.value = Value::Flags(MetadataFlags1(flags as _));
    }
}

/// An `area_effect_cloud` coming into view, as BDS announced the dragon's.
fn announcement(id: u64, at: [f32; 3], yaw: f32, spec: &Spec) -> Vec<Bytes> {
    let options = AddEntityOptions { id, identifier: KIND, feet: at, yaw, pitch: 0.0, variant: 0, mark_variant: 0, color: 0, baby: false, flags: MetadataFlags1(0), health: None, name: "" };
    let mut added: AddEntity = sapling_data::add_entity(&options);
    let items = vec![
        item(Key::Health, Plain::Int(1)),
        item(Key::OwnerEid, Plain::Long(-1)),
        item(Key::PotionColor, Plain::Int(spec.colour)),
        item(Key::PotionAuxValue, Plain::Short(spec.potion)),
        item(Key::BoundingboxWidth, Plain::Float(SIZE[0])),
        item(Key::BoundingboxHeight, Plain::Float(SIZE[1])),
        item(Key::AreaEffectCloudRadius, Plain::Float(spec.radius)),
        item(Key::AreaEffectCloudParticleId, Plain::Int(PARTICLE)),
        item(Key::AreaEffectCloudDuration, Plain::Int(spec.ticks as i32)),
        item(Key::AreaEffectCloudChangeRate, Plain::Float(spec.per_tick)),
        item(Key::AreaEffectCloudChangeOnPickup, Plain::Float(SHRINK_ON_PICKUP)),
        item(Key::AreaEffectCloudPickupCount, Plain::Int(0)),
    ];
    with(&mut added.metadata, FLAGS, items);
    let again = SetEntityData { runtime_entity_id: id, metadata: added.metadata.clone(), properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 };
    vec![sapling_net::encode(&AddEntity { head_yaw: 0.0, body_yaw: 0.0, ..added }), sapling_net::encode(&again)]
}

/// A cloud of `spec` at `at`, turned `yaw`; a player it kills is slain by `slayer`.
pub(crate) fn spawn(world: &mut World, layer: Entity, at: [f32; 3], yaw: f32, spec: Spec, slayer: &str) -> Option<Entity> {
    let cloud = entities::spawn(world, layer, KIND, Body::new(at, [0.0; 3], SIZE), at, |id| Some(announcement(id, at, yaw, &spec)))?;
    world.entity_mut(cloud).insert(Cloud { spec, slayer: slayer.to_owned(), born: None, used: 0.0, spared: HashMap::new() });
    Some(cloud)
}

/// The players of `layer` and its mobs a cloud may give to, their feet, and whether each is a player.
fn bodies(world: &mut World, layer: Entity) -> Vec<(Entity, [f32; 3], bool)> {
    let players = in_world(world).into_iter().filter(|player| world.get::<InLayer>(*player).is_some_and(|on| on.0 == layer));
    let mut all: Vec<(Entity, [f32; 3], bool)> = players.filter_map(|player| Some((player, world.get::<Position>(player)?.feet, true))).collect();
    let mut mobs = world.query_filtered::<(Entity, &InLayer, &Body, &entities::Kind), With<crate::creatures::Creature>>();
    all.extend(mobs.iter(world).filter(|(_, on, _, kind)| on.0 == layer && !UNAFFECTED.contains(&kind.0.as_str())).map(|(mob, _, body, _)| (mob, body.at, false)));
    all
}

fn radius(cloud: &Cloud, age: u64) -> f32 {
    cloud.spec.radius + cloud.spec.per_tick * age as f32 + cloud.used
}

fn pulse(world: &mut World, cloud: Entity, layer: Entity, now: u64) -> Option<()> {
    let at = world.get::<Body>(cloud)?.at;
    let kept = world.get::<Cloud>(cloud)?;
    let Some(born) = kept.born else {
        world.get_mut::<Cloud>(cloud)?.born = Some(now);
        return Some(());
    };
    let (spec, age, slayer) = (kept.spec, now - born, kept.slayer.clone());
    let reach = radius(kept, age);
    if age > spec.ticks || reach < SMALLEST {
        mobs::remove(world, cloud);
        return None;
    }
    if age % PULSE != 0 {
        return Some(());
    }
    world.get_mut::<Cloud>(cloud)?.spared.retain(|_, until| *until > age);
    for (victim, feet, is_player) in bodies(world, layer) {
        let near = (0..3).map(|axis| (feet[axis] - at[axis]).powi(2)).sum::<f32>() <= reach * reach;
        if !near || world.get::<Cloud>(cloud)?.spared.contains_key(&victim) {
            continue;
        }
        let mut kept = world.get_mut::<Cloud>(cloud)?;
        kept.spared.insert(victim, age + spec.again);
        kept.used += spec.on_use;
        match is_player {
            true => crate::vitals::clouded(world, victim, spec.potion, [TIME_SHARE, INSTANT_SHARE], &slayer, at),
            false => crate::creatures::clouded(world, victim, layer, spec.potion, [TIME_SHARE, INSTANT_SHARE]),
        }
        if world.get::<Cloud>(cloud).is_some_and(|kept| radius(kept, age) < SMALLEST) {
            mobs::remove(world, cloud);
            return None;
        }
    }
    Some(())
}

/// A tick of every cloud.
pub(crate) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let clouds: Vec<(Entity, Entity)> = world.query_filtered::<(Entity, &InLayer), With<Cloud>>().iter(world).map(|(cloud, layer)| (cloud, layer.0)).collect();
    for (cloud, layer) in clouds {
        pulse(world, cloud, layer, now);
    }
}
