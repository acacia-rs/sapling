//! A villager's bed (its pack's `minecraft:dweller`, `behavior.move_towards_dwelling_restriction`
//! and `behavior.sleep`): at its looks (jobs.rs) one without a bed claims the nearest free bed's
//! head as it does a job site; its schedule (schedule.rs) takes it home to it, and at bed time it
//! lies on it until its schedule wakes it.
// TODO: BDS's bed claim is a village POI, not decompiled; a sleeper here rests on its bed's head.

use acacia_proto::packets::SetEntityData;
use acacia_proto::types::{
    EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value,
    MetadataDictionaryItemValueDefault as Plain, MetadataFlags2,
};
use bevy_ecs::prelude::*;
use sapling_world::World as Level;

use super::Trader;
use super::jobs::{self, Claimed};
use crate::body::Body;
use crate::components::{InLayer, NetId};
use crate::entities::{self, Kind};

const BED: &str = "minecraft:bed";
/// `minecraft:movement` 0.5 times `behavior.sleep`'s `speed_multiplier` 0.6.
const GOES: f32 = 0.3;
/// How near its bed's head a villager lies down.
const LIES: f32 = 0.6;

fn is_head(level: &Level, [x, y, z]: [i32; 3]) -> bool {
    level.registry().get(level.block(x, y, z)).and_then(|state| state.property("head_piece_bit")) == Some("1")
}

/// At a villager's look: it gives up a bed that went, and one without a bed claims one.
pub(super) fn decide(level: &Level, claimed: &mut Claimed, trader: &mut Trader, (entity, layer, feet, now): (Entity, Entity, [i32; 3], u64)) {
    match trader.bed {
        Some(bed) if jobs::block_at(level, bed) != Some(BED) => {
            claimed.sites.remove(&(layer, bed));
            trader.bed = None;
        }
        None if claimed.may_search(now, jobs::FOR_BEDS) => {
            let fits = |cell, name: &str| (name == BED && is_head(level, cell)).then_some(0);
            if let Some((bed, _)) = jobs::nearest(level, claimed, layer, feet, fits) {
                claimed.sites.insert((layer, bed), entity);
                trader.bed = Some(bed);
            }
        }
        _ => {}
    }
}

/// The middle of the top of the villager's bed, if it has one.
pub(super) fn own(world: &World, entity: Entity) -> Option<[f32; 3]> {
    let [x, y, z] = world.get::<Trader>(entity)?.bed?;
    Some([x as f32 + 0.5, y as f32 + 0.5625, z as f32 + 0.5])
}

/// Walks the villager to its bed at `speed`, and keeps it there: false without one.
pub(super) fn go_to(world: &mut World, entity: Entity, layer: Entity, body: &Body, speed: f32, near: f32) -> bool {
    let Some(bed) = own(world, entity) else { return false };
    if (bed[0] - body.at[0]).hypot(bed[2] - body.at[2]) <= near {
        super::super::legs::rest(world, entity);
    } else {
        super::super::hunting::approach(world, entity, layer, body, bed, speed, near);
    }
    true
}

/// At bed time the villager goes to bed and sleeps there: false without one.
pub(super) fn sleep(world: &mut World, entity: Entity, layer: Entity, body: &Body) -> bool {
    let near = own(world, entity).is_some_and(|bed| (bed[0] - body.at[0]).hypot(bed[2] - body.at[2]) <= LIES);
    // Once down it stays down till bed time ends: a sleeper nudged off LIES got up and lay again.
    if near {
        lie(world, entity, true);
    }
    go_to(world, entity, layer, body, GOES, LIES)
}

/// A villager lying on its bed.
#[derive(Component)]
pub(super) struct Lying;

/// What BDS showed of a villager lying down and getting up: a 1 by 0.3 box and `FlagsExtended` bit
/// 12 over its usual bits 17 and 25, then its own box back; the bed's `occupied_bit` stays 0
/// (research/captures/bds-village.txt).
const LYING_BOX: [f32; 2] = [1.0, 0.3];
const EXTENDED: i64 = (1 << 25) | (1 << 17);
const SLEEPING: i64 = 1 << 12;

/// Lays the villager down or gets it up, telling its viewers when that changes.
pub(super) fn lie(world: &mut World, entity: Entity, lying: bool) {
    if world.get::<Lying>(entity).is_some() == lying {
        return;
    }
    match lying {
        true => world.entity_mut(entity).insert(Lying),
        false => world.entity_mut(entity).remove::<Lying>(),
    };
    let (Some(id), Some(layer), Some(kind), Some(body)) = (world.get::<NetId>(entity).map(|id| id.0), world.get::<InLayer>(entity).map(|on| on.0), world.get::<Kind>(entity).map(|kind| kind.0.clone()), world.get::<Body>(entity).copied()) else { return };
    let baby = super::super::shown::is_baby(world, entity);
    let [width, height] = if lying { LYING_BOX } else { sapling_data::mob_look(&kind, baby).map_or([0.6, 1.9], |(size, _)| size) };
    let size = |key, value| MetadataDictionaryItem { key, r#type: Type::Float, legacy_type: 3, value: Value::Default(Plain::Float(value)) };
    let flags = MetadataDictionaryItem { key: Key::FlagsExtended, r#type: Type::Long, legacy_type: 7, value: Value::FlagsExtended(MetadataFlags2(EXTENDED | if lying { SLEEPING } else { 0 })) };
    let data = SetEntityData { runtime_entity_id: id, metadata: vec![size(Key::BoundingboxWidth, width), size(Key::BoundingboxHeight, height), flags], properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 };
    entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), vec![sapling_net::encode(&data)]);
}
