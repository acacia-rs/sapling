//! The dragon, as BDS made one (research/captures/bds-dragon-watch.txt): at (0, 128, 0), facing
//! north, with its announced pitch at random; its metadata and attributes are told again at once,
//! after the sound of its coming from its head. One a world folder kept is the fight's again.

use acacia_proto::packets::{AddEntity, LevelSoundEvent, SetEntityData, UpdateAttributes};
use acacia_proto::types::{EntityProperties, PlayerAttributesItem, Vec3f};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_data::AddEntityOptions;

use super::random::Random;
use crate::body::Body;
use crate::components::{InLayer, Tick};
use crate::creatures::{Creature, Health};
use crate::entities;

pub(crate) const KIND: &str = "minecraft:ender_dragon";
const AT: [f32; 3] = [0.0, 128.0, 0.0];
/// Where its head is: up and ahead of its feet.
const HEAD_UP: f32 = 1.0;
const HEAD_AHEAD: f32 = 5.6875;

/// The fight's dragon: when it came, its flight, and what its viewers were told of it.
#[derive(Component)]
pub(crate) struct Dragon {
    pub born: u64,
    pub flight: super::flight::Flight,
    pub shown: super::wire::Shown,
}

fn announcement(id: u64, pitch: f32, body: &Body, yaw: f32, health: Option<f32>) -> Vec<Bytes> {
    let options = AddEntityOptions { id, identifier: KIND, feet: body.at, yaw, pitch, variant: 0, mark_variant: 0, color: 0, baby: false, flags: acacia_proto::types::MetadataFlags1(0), health, name: "" };
    let added: AddEntity = sapling_data::add_entity(&options);
    let data = SetEntityData { runtime_entity_id: id, metadata: added.metadata.clone(), properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 };
    let attributes = added.attributes.iter().map(|a| PlayerAttributesItem { name: a.name.clone(), min: a.min, max: a.max, current: a.value, default_min: a.min, default_max: a.max, default: a.value, modifiers: Vec::new() }).collect();
    let attributes = UpdateAttributes { runtime_entity_id: id, attributes, tick: 0 };
    let added = AddEntity { yaw: 0.0, head_yaw: yaw, body_yaw: yaw, ..added };
    vec![sapling_net::encode(&added), sapling_net::encode(&data), sapling_net::encode(&attributes)]
}

/// The sound of a dragon coming, from its head.
fn sound(id: u64, body: &Body, yaw: f32) -> Bytes {
    let (sin, cos) = yaw.to_radians().sin_cos();
    let position = Vec3f { x: body.at[0] - sin * HEAD_AHEAD, y: body.at[1] + HEAD_UP, z: body.at[2] - cos * HEAD_AHEAD };
    sapling_net::encode(&LevelSoundEvent { sound_id: "spawn".into(), position, extra_data: -1, entity_type: KIND.into(), is_baby_mob: false, is_global: false, entity_unique_id: id as i64, fire_at_position: None })
}

/// A new dragon over the island; its stored id for the fight's record.
pub(super) fn new(world: &mut World, layer: Entity, random: &mut Random) -> Option<(Entity, i64)> {
    let size = sapling_data::mob_size(KIND)?;
    let max = sapling_data::mob_health(KIND)?;
    let body = Body::new(AT, [0.0; 3], size);
    let pitch = random.next_float() * 360.0 - 180.0;
    let entity = entities::spawn(world, layer, KIND, body, AT, |id| {
        let mut told = announcement(id, pitch, &body, 0.0, None);
        told.insert(1, sound(id, &body, 0.0));
        Some(told)
    })?;
    let born = world.resource::<Tick>().0;
    let dragon = Dragon { born, flight: super::flight::Flight::new(AT, pitch), shown: super::wire::Shown::announced(AT) };
    world.entity_mut(entity).insert((Creature::plain(), Health::new(max, None, 0), dragon));
    let unique_id = entities::new_unique_id(world, entity)?;
    entities::keep(world, entity, layer, unique_id, sapling_world::new_actor(unique_id, KIND, AT, [0.0; 3]));
    Some((entity, unique_id))
}

/// Makes a dragon a world folder kept the fight's.
pub(crate) fn adopt(world: &mut World, entity: Entity) {
    let (born, at) = (world.resource::<Tick>().0, world.get::<Body>(entity).map_or(AT, |body| body.at));
    world.entity_mut(entity).insert(Dragon { born, flight: super::flight::Flight::new(at, 0.0), shown: super::wire::Shown::announced(at) });
}

/// A dragon of `layer` within `reach` of the island's middle every way, if one is there.
pub(super) fn find(world: &mut World, layer: Entity, reach: f32) -> Option<Entity> {
    let near = |body: &Body| body.at.iter().all(|c| c.abs() <= reach);
    world.query_filtered::<(Entity, &InLayer, &Body), With<Dragon>>().iter(world).find(|(_, in_layer, body)| in_layer.0 == layer && near(body)).map(|(dragon, ..)| dragon)
}
