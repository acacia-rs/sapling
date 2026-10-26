//! The dragon's breath clouds (`area_effect_cloud`, research/dragon-extras-decompile.md 4.3-4.5,
//! research/captures/bds-dragon-survival.txt): one where a fireball meets something, radius 6 for
//! 2400 ticks, and one by its head as it flames, radius 5 for 200. Neither grows nor shrinks. From
//! the tick after it comes, every fifth tick of its age it harms each player and mob whose feet are
//! within its radius by half an instant harm (3; the undead are mended), a player thrown as by a
//! magic hit from the cloud; the fireball's spares one it harmed for 20 ticks.
// BDS's pulse stops at its owner (extras 5.4), but these clouds have none (OwnerEid -1 as captured).

use std::collections::HashMap;

use acacia_proto::packets::{AddEntity, SetEntityData};
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::GameMode;
use sapling_data::AddEntityOptions;

use crate::body::Body;
use crate::components::{InLayer, Mode, Position, Tick, in_world};
use crate::{entities, mobs};

pub(crate) const KIND: &str = "minecraft:area_effect_cloud";
pub(crate) const SIZE: [f32; 2] = [0.6, 1.8];
const FLAGS: u64 = 0x6_0000_0000_0000;
const HARMING: i16 = 23;
const PARTICLE: i32 = 49;
const PULSE: u64 = 5;
const HARM: f32 = 3.0;
/// Of a potion's instant harm, as `applyInstantaneousEffect(.., 0.5)` gives it.
const HARM_SHARE: f32 = 0.5;
const SHRINK_ON_PICKUP: f32 = -0.5;

/// A cloud's look and reach.
#[derive(Clone, Copy, Debug)]
pub(super) struct Breath {
    pub radius: f32,
    pub ticks: u64,
    pub colour: i32,
    /// Ticks one it harmed is spared.
    pub spares: u64,
}

/// Where a fireball meets something.
pub(super) const FIREBALL: Breath = Breath { radius: 6.0, ticks: 2400, colour: -5_675_670, spares: 20 };
/// By the flaming dragon's head.
pub(super) const FLAMES: Breath = Breath { radius: 5.0, ticks: 200, colour: 14_418_159, spares: 0 };

#[derive(Component)]
struct Cloud {
    breath: Breath,
    born: Option<u64>,
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

/// How a cloud is shown: its reach, its life, its colour, its potion and how fast it shrinks.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Shown {
    pub radius: f32,
    pub ticks: u64,
    pub colour: i32,
    pub potion: i16,
    pub shrinks: f32,
}

/// An `area_effect_cloud` coming into view, as BDS announced the dragon's.
pub(crate) fn announcement(id: u64, at: [f32; 3], yaw: f32, shown: Shown) -> Vec<Bytes> {
    let options = AddEntityOptions { id, identifier: KIND, feet: at, yaw, pitch: 0.0, variant: 0, mark_variant: 0, color: 0, baby: false, flags: MetadataFlags1(0), health: None, name: "" };
    let mut added: AddEntity = sapling_data::add_entity(&options);
    let items = vec![
        item(Key::Health, Plain::Int(1)),
        item(Key::OwnerEid, Plain::Long(-1)),
        item(Key::PotionColor, Plain::Int(shown.colour)),
        item(Key::PotionAuxValue, Plain::Short(shown.potion)),
        item(Key::BoundingboxWidth, Plain::Float(SIZE[0])),
        item(Key::BoundingboxHeight, Plain::Float(SIZE[1])),
        item(Key::AreaEffectCloudRadius, Plain::Float(shown.radius)),
        item(Key::AreaEffectCloudParticleId, Plain::Int(PARTICLE)),
        item(Key::AreaEffectCloudDuration, Plain::Int(shown.ticks as i32)),
        item(Key::AreaEffectCloudChangeRate, Plain::Float(shown.shrinks)),
        item(Key::AreaEffectCloudChangeOnPickup, Plain::Float(SHRINK_ON_PICKUP)),
        item(Key::AreaEffectCloudPickupCount, Plain::Int(0)),
    ];
    with(&mut added.metadata, FLAGS, items);
    let again = SetEntityData { runtime_entity_id: id, metadata: added.metadata.clone(), properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 };
    vec![sapling_net::encode(&AddEntity { head_yaw: 0.0, body_yaw: 0.0, ..added }), sapling_net::encode(&again)]
}

/// A cloud of `breath` at `at`, turned `yaw`.
pub(super) fn spawn(world: &mut World, layer: Entity, at: [f32; 3], yaw: f32, breath: Breath) -> Option<Entity> {
    let shown = Shown { radius: breath.radius, ticks: breath.ticks, colour: breath.colour, potion: HARMING, shrinks: -0.0 };
    let cloud = entities::spawn(world, layer, KIND, Body::new(at, [0.0; 3], SIZE), at, |id| Some(announcement(id, at, yaw, shown)))?;
    world.entity_mut(cloud).insert(Cloud { breath, born: None, spared: HashMap::new() });
    Some(cloud)
}

/// The players of `layer` it may harm, and their feet.
fn mortals(world: &World, layer: Entity) -> Vec<(Entity, [f32; 3])> {
    let mortal = |player: &Entity| world.get::<InLayer>(*player).is_some_and(|in_layer| in_layer.0 == layer) && world.get::<Mode>(*player).is_some_and(|mode| matches!(mode.0, GameMode::Survival | GameMode::Adventure));
    in_world(world).into_iter().filter(mortal).filter_map(|player| Some((player, world.get::<Position>(player)?.feet))).collect()
}

fn pulse(world: &mut World, cloud: Entity, layer: Entity, now: u64) -> Option<()> {
    let at = world.get::<Body>(cloud)?.at;
    let kept = world.get::<Cloud>(cloud)?;
    let born = match kept.born {
        Some(born) => born,
        None => {
            world.get_mut::<Cloud>(cloud)?.born = Some(now);
            return Some(());
        }
    };
    let (breath, age) = (kept.breath, now - born);
    if age > breath.ticks {
        mobs::remove(world, cloud);
        return None;
    }
    if age % PULSE != 0 {
        return Some(());
    }
    world.get_mut::<Cloud>(cloud)?.spared.retain(|_, until| *until > age);
    let mobs: Vec<(Entity, [f32; 3])> = world.query_filtered::<(Entity, &InLayer, &Body), (With<crate::creatures::Creature>, Without<super::Dragon>)>().iter(world).filter(|(_, in_layer, _)| in_layer.0 == layer).map(|(mob, _, body)| (mob, body.at)).collect();
    let victims = mortals(world, layer).into_iter().map(|(player, feet)| (player, feet, true)).chain(mobs.into_iter().map(|(mob, feet)| (mob, feet, false)));
    for (victim, feet, is_player) in victims.collect::<Vec<_>>() {
        let near = (0..3).map(|axis| (feet[axis] - at[axis]).powi(2)).sum::<f32>() <= breath.radius * breath.radius;
        if !near || world.get::<Cloud>(cloud)?.spared.contains_key(&victim) {
            continue;
        }
        world.get_mut::<Cloud>(cloud)?.spared.insert(victim, age + breath.spares);
        match is_player {
            true => {
                crate::vitals::fanged(world, victim, HARM, super::DRAGON, at);
            }
            false => crate::creatures::breathed_on(world, victim, layer, HARM_SHARE),
        }
    }
    Some(())
}

/// A tick of every cloud.
pub(super) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let clouds: Vec<(Entity, Entity)> = world.query_filtered::<(Entity, &InLayer), With<Cloud>>().iter(world).map(|(cloud, layer)| (cloud, layer.0)).collect();
    for (cloud, layer) in clouds {
        pulse(world, cloud, layer, now);
    }
}
