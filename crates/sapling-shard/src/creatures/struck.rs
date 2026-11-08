//! What lightning does to a mob, as BDS's packs and `Actor::onLightningHit` have it: a pig becomes
//! a zombified piglin and a villager a witch, a mooshroom turns red or brown, a creeper is charged
//! (its blast twice as strong, blast.rs), all four unhurt; a turtle dies; anything else takes 5
//! and, unless fire cannot hurt it, burns 8 seconds. A pig or a villager turns 10 ticks after the
//! strike (its pack's `transformation` delay 0.5), the pig with `mob.pig.death` at its middle
//! (research/captures/bds-n3struck.txt).

use acacia_proto::packets::SetEntityData;
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1};
use bevy_ecs::prelude::*;
use sapling_api::SpawnReason;

use super::health::{Cause, Health};
use super::turning::Turning;
use super::{genes, harm};
use crate::body::Body;
use crate::components::NetId;
use crate::entities;

const CREEPER: &str = "minecraft:creeper";
const CHARGED_GROUP: &str = "minecraft:charged_creeper";
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

/// Charges the creeper, shown so to those who come to see it later; the packet that tells its viewers now.
fn charged(world: &mut World, mob: Entity) -> Option<SetEntityData> {
    let id = world.get::<NetId>(mob)?.0;
    world.entity_mut(mob).insert(Charged);
    let flags = entities::mob_flags(world, mob, CREEPER, MetadataFlags1::POWERED, true);
    let told = sapling_data::mob_flags_set(id, flags);
    crate::mobs::reannounce_item(world, mob, told.metadata[0].clone());
    Some(told)
}

fn charge(world: &mut World, mob: Entity, layer: Entity) -> Option<()> {
    let told = charged(world, mob)?;
    entities::restore(world, mob, |compound, _| sapling_world::regrouped_actor(compound, &[], &[CHARGED_GROUP]).ok());
    tell(world, mob, layer, vec![sapling_net::encode(&told)])
}

/// A creeper's `minecraft:become_charged` charges it as lightning does (bds-bg2names.txt: POWERED at the sync).
pub(super) fn evented(world: &mut World, mob: Entity, kind: &str, event: &str) {
    if kind != CREEPER || event != "minecraft:become_charged" || world.get::<Charged>(mob).is_some() {
        return;
    }
    if let Some(layer) = world.get::<crate::components::InLayer>(mob).map(|in_layer| in_layer.0) {
        charge(world, mob, layer);
    }
}

/// A charged creeper's flags, which its announcement shows once rebuilt.
pub(super) fn shown_flags(world: &World, mob: Entity) -> Option<MetadataDictionaryItem> {
    world.get::<Charged>(mob)?;
    let flags = world.get::<crate::sync::Told>(mob).and_then(crate::sync::Told::mob_flags).unwrap_or_else(|| sapling_data::grown_flags(CREEPER, MetadataFlags1::POWERED));
    let id = world.get::<NetId>(mob)?.0;
    sapling_data::mob_flags_set(id, flags).metadata.pop()
}

/// A creeper kept charged (BDS's `minecraft:charged_creeper` group) is charged again when loaded.
pub(super) fn adopt(world: &mut World, mob: Entity) {
    let creeper = world.get::<entities::Kind>(mob).is_some_and(|kind| kind.0 == CREEPER);
    let kept = world.get::<entities::Stored>(mob).is_some_and(|stored| sapling_world::actor_groups(&stored.compound).iter().any(|group| group == CHARGED_GROUP));
    if creeper && kept {
        charged(world, mob);
    }
}

/// Lightning struck the mob.
pub(crate) fn struck_by_lightning(world: &mut World, mob: Entity, layer: Entity) {
    let Some(kind) = world.get::<entities::Kind>(mob).map(|kind| kind.0.clone()) else { return };
    if let Some(&(_, into, sound)) = TURNS.iter().find(|(from, ..)| *from == kind) {
        if world.get::<Turning>(mob).is_none() {
            let at = world.resource::<crate::components::Tick>().0 + TURN_DELAY;
            world.entity_mut(mob).insert(Turning { into, sound, at, keeps_gear: false, reason: SpawnReason::Lightning });
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
