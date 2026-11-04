//! What lightning does to a mob, as BDS's packs and `Actor::onLightningHit` have it: a pig becomes
//! a zombified piglin and a villager a witch, a mooshroom turns red or brown, a creeper is charged
//! (its blast twice as strong, blast.rs), all four unhurt; a turtle dies; anything else takes 5
//! and, unless fire cannot hurt it, burns 8 seconds. A pig or a villager turns 10 ticks after the
//! strike (its pack's `transformation` delay 0.5), the pig with `mob.pig.death` at its middle
//! (research/captures/bds-n3struck.txt).

use acacia_proto::packets::{LevelSoundEvent, SetEntityData};
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1};
use bevy_ecs::prelude::*;
use sapling_api::SpawnReason;

use super::health::{Cause, Health};
use super::{genes, harm, legs};
use crate::body::Body;
use crate::components::NetId;
use crate::{entities, mobs};

const CREEPER: &str = "minecraft:creeper";
const MOOSHROOM: &str = "minecraft:mooshroom";
const TURTLE: &str = "minecraft:turtle";
/// A turtle takes lightning this many times over.
const TURTLE_TAKES: f32 = 2000.0;
const BROWN: i32 = 1;
/// What each kind turns into, and the sound of its turning.
const TURNS: &[(&str, &str, Option<&str>)] = &[
    ("minecraft:pig", "minecraft:zombie_pigman", Some("mob.pig.death")),
    ("minecraft:villager_v2", "minecraft:witch", None),
    ("minecraft:villager", "minecraft:witch", None),
];
const TURN_DELAY: u64 = 10;

/// A creeper lightning struck.
#[derive(Component)]
pub(super) struct Charged;

/// A mob lightning struck, turning into another at the tick `at`.
#[derive(Component)]
pub(super) struct Turning {
    into: &'static str,
    sound: Option<&'static str>,
    at: u64,
}

/// The mob takes the place of one of `kind` facing the same way, where it stood, heard as `sound`.
fn turn_into(world: &mut World, mob: Entity, layer: Entity, kind: &str, sound: Option<&str>) -> Option<()> {
    let body = *world.get::<Body>(mob)?;
    let yaw = world.get::<legs::Legs>(mob).map_or(0.0, |legs| legs.yaw);
    let turned = super::spawn_as(world, layer, kind, body.at, SpawnReason::Lightning, yaw, None).and_then(|id| entities::find(world, id));
    mobs::remove(world, mob);
    let (Some(turned), Some(sound)) = (turned, sound) else { return Some(()) };
    let position = acacia_proto::types::Vec3f { x: body.at[0], y: body.at[1] + body.size[1] / 2.0, z: body.at[2] };
    let heard = LevelSoundEvent { sound_id: sound.into(), position, extra_data: -1, entity_type: String::new(), is_baby_mob: false, is_global: false, entity_unique_id: -1, fire_at_position: None };
    tell(world, turned, layer, vec![sapling_net::encode(&heard)])
}

/// A tick: struck mobs whose time came turn.
pub(super) fn tick(world: &mut World) {
    let now = world.resource::<crate::components::Tick>().0;
    let due: Vec<(Entity, Entity, &'static str, Option<&'static str>)> = world
        .query::<(Entity, &Turning, &crate::components::InLayer)>()
        .iter(world)
        .filter(|(_, turning, _)| turning.at <= now)
        .map(|(mob, turning, layer)| (mob, layer.0, turning.into, turning.sound))
        .collect();
    for (mob, layer, into, sound) in due {
        turn_into(world, mob, layer, into, sound);
    }
}

fn tell(world: &mut World, mob: Entity, layer: Entity, packets: Vec<bytes::Bytes>) -> Option<()> {
    let at = world.get::<Body>(mob)?.at;
    entities::tell_viewers(world, mob, layer, at.map(|c| c.floor() as i32), packets);
    Some(())
}

/// The mooshroom turns brown, or red again, and its viewers are told its `Variant`.
fn recolour(world: &mut World, mob: Entity, layer: Entity) -> Option<()> {
    let id = world.get::<NetId>(mob)?.0;
    let variant = if world.get::<genes::Gene>(mob).is_some_and(|gene| gene.0 == BROWN) { 0 } else { BROWN };
    world.entity_mut(mob).insert(genes::Gene(variant));
    let item = MetadataDictionaryItem { key: Key::Variant, r#type: Type::Int, legacy_type: 2, value: Value::Default(Plain::Int(variant)) };
    let told = SetEntityData { runtime_entity_id: id, metadata: vec![item], properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 };
    let body = *world.get::<Body>(mob)?;
    tell(world, mob, layer, vec![sapling_net::encode(&told), super::hunting::sound("convert_mooshroom", id, MOOSHROOM, &body, 0.5)])
}

fn charge(world: &mut World, mob: Entity, layer: Entity) -> Option<()> {
    let id = world.get::<NetId>(mob)?.0;
    world.entity_mut(mob).insert(Charged);
    let flags = entities::mob_flags(world, mob, CREEPER, MetadataFlags1::POWERED, true);
    tell(world, mob, layer, vec![sapling_net::encode(&sapling_data::mob_flags_set(id, flags))])
}

/// Lightning struck the mob.
pub(crate) fn struck_by_lightning(world: &mut World, mob: Entity, layer: Entity) {
    let Some(kind) = world.get::<entities::Kind>(mob).map(|kind| kind.0.clone()) else { return };
    if let Some(&(_, into, sound)) = TURNS.iter().find(|(from, ..)| *from == kind) {
        if world.get::<Turning>(mob).is_none() {
            let at = world.resource::<crate::components::Tick>().0 + TURN_DELAY;
            world.entity_mut(mob).insert(Turning { into, sound, at });
        }
        return;
    }
    match kind.as_str() {
        MOOSHROOM => drop(recolour(world, mob, layer)),
        CREEPER => drop(charge(world, mob, layer)),
        TURTLE => drop(harm::wound(world, mob, layer, crate::lightning::HARM * TURTLE_TAKES, Cause::Lightning)),
        _ => {
            if let Some(mut health) = world.get_mut::<Health>(mob).filter(|health| !health.is_fireproof()) {
                health.ignite(crate::lightning::BURNS);
            }
            harm::wound(world, mob, layer, crate::lightning::HARM, Cause::Lightning);
        }
    }
}
