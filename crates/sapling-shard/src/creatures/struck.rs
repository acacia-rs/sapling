//! What lightning does to a mob, as BDS's packs and `Actor::onLightningHit` have it: a pig becomes
//! a zombified piglin and a villager a witch, a mooshroom turns red or brown, a creeper is charged
//! (its blast twice as strong, blast.rs), all four unhurt; a turtle dies; anything else takes 5
//! and, unless fire cannot hurt it, burns 8 seconds.
// TODO: BDS turns a pig or a villager half a second after the bolt, not at once.

use acacia_proto::packets::SetEntityData;
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
const TURNS: &[(&str, &str)] = &[("minecraft:pig", "minecraft:zombie_pigman"), ("minecraft:villager_v2", "minecraft:witch"), ("minecraft:villager", "minecraft:witch")];

/// A creeper lightning struck.
#[derive(Component)]
pub(super) struct Charged;

/// The mob takes the place of one of `kind` facing the same way, where it stood.
fn turn_into(world: &mut World, mob: Entity, layer: Entity, kind: &str) -> Option<()> {
    let body = *world.get::<Body>(mob)?;
    let yaw = world.get::<legs::Legs>(mob).map_or(0.0, |legs| legs.yaw);
    mobs::remove(world, mob);
    super::spawn_as(world, layer, kind, body.at, SpawnReason::Lightning, yaw, None).map(drop)
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
    if let Some((_, into)) = TURNS.iter().find(|(from, _)| *from == kind) {
        turn_into(world, mob, layer, into);
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
