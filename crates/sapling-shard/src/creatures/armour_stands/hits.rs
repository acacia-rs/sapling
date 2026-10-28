//! Armour stands knocked over, as BDS's were (research/captures/bds-standhits.txt): a survival
//! hit is heard (`SoundArmorStandHit`) and wobbles the stand, its `HurtTime` 9 counting down a
//! tick at a time and its `HurtDirection` turning over; a second hit soon after, or one creative
//! hit, breaks it with `ParticleDestroyArmorStand` and `SoundArmorStandBreak` at its feet. A
//! survival break drops the stand 1.125 above its feet and its gear about its box.
// TODO: how soon "soon" is was not drilled between 0 ticks (broke) and 40 (did not); 5 is Java's.

use acacia_proto::packets::{LevelEvent, LevelEventEvent, SetEntityData};
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, Vec3f};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_world::ItemStack;

use super::{Gear, STAND, gear_of};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, NetId, Tick};
use crate::{drops, entities, mobs};

const AGAIN_WITHIN: u64 = 5;
const WOBBLES: i32 = 9;
/// What BDS showed as its health on its first hit.
const FIRST_HEALTH: i32 = -10;
const DROP_ABOVE: f32 = 1.125;
/// Its gear comes out this far across from its feet, anywhere up its height.
const GEAR_ACROSS: f32 = 0.25;
const HEIGHT: f32 = 1.975;

/// A hit stand's wobble: the tick it was last hit in, how long it still wobbles, and the way.
#[derive(Component)]
pub(in crate::creatures) struct Wobble {
    hit: u64,
    left: i32,
    direction: i32,
}

fn data(id: u64, items: Vec<(Key, i32)>) -> Bytes {
    let metadata = items.into_iter().map(|(key, value)| MetadataDictionaryItem { key, r#type: Type::Int, legacy_type: 2, value: Value::Default(Plain::Int(value)) }).collect();
    sapling_net::encode(&SetEntityData { runtime_entity_id: id, metadata, properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 })
}

fn event(event: LevelEventEvent, [x, y, z]: [f32; 3], data: i32) -> Bytes {
    sapling_net::encode(&LevelEvent { event, position: Vec3f { x, y, z }, data })
}

/// `stand` was hit by a player, `creative` or not: true when it is an armour stand.
pub(in crate::creatures) fn struck(world: &mut World, stand: Entity, kind: &str, creative: bool, mut told: Vec<Bytes>) -> bool {
    if kind != STAND {
        return false;
    }
    let (Some(id), Some(layer), Some(body)) = (world.get::<NetId>(stand).map(|id| id.0), world.get::<InLayer>(stand).map(|layer| layer.0), world.get::<Body>(stand).copied()) else { return true };
    let now = world.resource::<Tick>().0;
    let cell = body.at.map(|c| c.floor() as i32);
    let last = world.get::<Wobble>(stand).map(|wobble| (wobble.hit, wobble.direction));
    if !creative {
        told.insert(0, event(LevelEventEvent::SoundArmorStandHit, body.at, 0));
    }
    if creative || last.is_some_and(|(hit, _)| now - hit <= AGAIN_WITHIN) {
        told.extend([event(LevelEventEvent::ParticleDestroyArmorStand, body.at, 1), event(LevelEventEvent::SoundArmorStandBreak, body.at, 0)]);
        entities::tell_viewers(world, stand, layer, cell, told);
        if !creative {
            drop_all(world, stand, layer, body.at);
        }
        mobs::remove(world, stand);
        return true;
    }
    let direction = -last.map_or(1, |(_, direction)| direction);
    let mut wobble = vec![(Key::HurtTime, WOBBLES), (Key::HurtDirection, direction)];
    if last.is_none() {
        wobble.insert(0, (Key::Health, FIRST_HEALTH));
    }
    told.push(data(id, wobble));
    world.entity_mut(stand).insert(Wobble { hit: now, left: WOBBLES, direction });
    entities::tell_viewers(world, stand, layer, cell, told);
    true
}

/// The stand item, then what it held and wore.
fn drop_all(world: &mut World, stand: Entity, layer: Entity, [x, y, z]: [f32; 3]) {
    let Gear { slots, .. } = gear_of(world, stand);
    drops::toss(world, layer, ItemStack { name: STAND.into(), count: 1, damage: 0, block: None, tag: None }, [x, y + DROP_ABOVE, z]);
    for stack in slots.into_iter().flatten() {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        let at = [x + (dice.fraction() * 2.0 - 1.0) * GEAR_ACROSS, y + dice.fraction() * HEIGHT, z + (dice.fraction() * 2.0 - 1.0) * GEAR_ACROSS];
        drops::toss(world, layer, stack, at);
    }
}

/// Wobbling stands count down, told each tick.
pub(in crate::creatures) fn tick(world: &mut World) {
    let wobbling: Vec<(Entity, u64, Entity, [f32; 3])> = world.query::<(Entity, &mut Wobble, &NetId, &InLayer, &Body)>().iter_mut(world).filter(|(_, wobble, ..)| wobble.left > 0).map(|(stand, mut wobble, id, layer, body)| {
        wobble.left -= 1;
        (stand, id.0, layer.0, body.at)
    }).collect();
    for (stand, id, layer, at) in wobbling {
        let left = world.get::<Wobble>(stand).map_or(0, |wobble| wobble.left);
        entities::tell_viewers(world, stand, layer, at.map(|c| c.floor() as i32), vec![data(id, vec![(Key::HurtTime, left)])]);
    }
}
