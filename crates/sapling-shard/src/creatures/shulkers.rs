//! A shulker's shell and its hops, as BDS's (research/captures/bds-shulker-findings.md,
//! bds-d3shulker.txt): open 100 while it has prey; with none it peeks open 30 for 19, 39 or 59
//! ticks, one tick in 40 on average (twelve gaps between peeks averaged 42, as the Java game's
//! `ShulkerPeekGoal` one in 40; three were exactly 10, unexplained); every change heard a tick
//! later. One whose floor goes hops at once to a free cell over a solid block within 8 each way
//! (Java `teleportSomewhere`, five tries), told its new `ShulkerAttachPos` and heard `teleport` where
//! it stood; BDS moved it the tick after the attach position, sapling in the same tick.
//! A hurt one does not hop, unlike the Java game's (bds-tt1shulker.txt: 40 `/damage` hurts, none).

use acacia_proto::packets::{LevelSoundEvent, SetEntityData};
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, Vec3f, Vec3i};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_world::World as Level;

use super::bullets::SHULKER;
use super::{Creature, Dying, hunting};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Tick};
use crate::{entities, late_sounds};

/// Open this far for prey, and this far for a peek.
const OPEN: i32 = 100;
const PEEK: i32 = 30;
/// A peek starts one tick in this many and lasts 20 ticks, so many times over, less one.
const PEEKS: u32 = 40;
const PEEK_TIMES: u32 = 3;
const HOP: i32 = 8;
const HOP_TRIES: usize = 5;
const SIZE: [f32; 2] = [1.0, 1.0];

/// An idle peek's end.
#[derive(Component)]
struct Peeking(u64);

fn heard(name: &str, [x, y, z]: [f32; 3], id: u64) -> Bytes {
    sapling_net::encode(&LevelSoundEvent { sound_id: name.into(), position: Vec3f { x, y, z }, extra_data: -1, entity_type: SHULKER.into(), is_baby_mob: false, is_global: false, entity_unique_id: id as i64, fire_at_position: None })
}

fn data(id: u64, metadata: Vec<MetadataDictionaryItem>) -> Bytes {
    sapling_net::encode(&SetEntityData { runtime_entity_id: id, metadata, properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 })
}

fn peek_item(open: i32) -> MetadataDictionaryItem {
    MetadataDictionaryItem { key: Key::ShulkerPeekId, r#type: Type::Int, legacy_type: 2, value: Value::Default(Plain::Int(open)) }
}

/// The shulker `id` opens `open` far (0 closes), heard a tick later.
fn opened(world: &mut World, layer: Entity, id: u64, feet: [f32; 3], open: i32) -> Bytes {
    let sound = heard(if open > 0 { "shulker.open" } else { "shulker.close" }, feet, id);
    late_sounds::play_later(world, 1, layer, feet.map(|c| c.floor() as i32), sound);
    data(id, vec![peek_item(open)])
}

/// The shulker `id` opens for its prey or closes, heard a tick later; nothing for other kinds.
pub(super) fn peek(world: &mut World, layer: Entity, id: u64, kind: &str, feet: [f32; 3], open: bool) -> Option<Bytes> {
    (kind == SHULKER).then(|| opened(world, layer, id, feet, if open { OPEN } else { 0 }))
}

fn holds_on(level: &Level, [x, y, z]: [i32; 3]) -> bool {
    level.in_height(y - 1) && level.registry().get(level.block(x, y - 1, z)).is_some_and(|under| under.is_solid())
}

/// A free cell over a solid block within [`HOP`] of `from`, if one of [`HOP_TRIES`] picks is.
fn hop_to(world: &mut World, layer: Entity, from: [i32; 3]) -> Option<[i32; 3]> {
    let picks: Vec<[i32; 3]> = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        (0..HOP_TRIES).map(|_| from.map(|c| c + dice.roll((2 * HOP + 1) as u32) as i32 - HOP)).collect()
    };
    let level = &world.get::<Layer>(layer)?.world;
    picks.into_iter().find(|cell| cell != &from && holds_on(level, *cell) && Body::new([cell[0] as f32 + 0.5, cell[1] as f32, cell[2] as f32 + 0.5], [0.0; 3], SIZE).is_free(level))
}

/// A tick of one shulker: it hops off a floor that went, and peeks when idle.
fn tend(world: &mut World, shulker: Entity, layer: Entity, id: u64, now: u64) -> Option<()> {
    let mut body = *world.get::<Body>(shulker)?;
    let cell = body.at.map(|c| c.floor() as i32);
    let mut told = Vec::new();
    if !holds_on(&world.get::<Layer>(layer)?.world, cell) {
        if let Some(to) = hop_to(world, layer, cell) {
            world.entity_mut(shulker).remove::<Peeking>();
            let attach = MetadataDictionaryItem { key: Key::ShulkerAttachPos, r#type: Type::Vec3i, legacy_type: 6, value: Value::Default(Plain::Vec3i(Vec3i { x: to[0], y: to[1], z: to[2] })) };
            told.extend([data(id, vec![peek_item(0), attach]), heard("teleport", body.at, id)]);
            body.at = [to[0] as f32 + 0.5, to[1] as f32, to[2] as f32 + 0.5];
            body.motion = [0.0; 3];
            *world.get_mut::<Body>(shulker)? = body;
        }
    } else if hunting::prey_at(world, shulker).is_some() {
        world.entity_mut(shulker).remove::<Peeking>();
    } else if let Some(until) = world.get::<Peeking>(shulker).map(|peeking| peeking.0) {
        if now >= until {
            world.entity_mut(shulker).remove::<Peeking>();
            told.push(opened(world, layer, id, body.at, 0));
        }
    } else {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        if dice.roll(PEEKS) == 0 {
            let lasts = 20 * u64::from(1 + dice.roll(PEEK_TIMES)) - 1;
            world.entity_mut(shulker).insert(Peeking(now + lasts));
            told.push(opened(world, layer, id, body.at, PEEK));
        }
    }
    if !told.is_empty() {
        entities::tell_viewers(world, shulker, layer, cell, told);
    }
    Some(())
}

/// A tick of every shulker.
pub(super) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let shulkers: Vec<(Entity, Entity, u64)> = world
        .query_filtered::<(Entity, &InLayer, &NetId, &entities::Kind), (With<Creature>, Without<Dying>)>()
        .iter(world)
        .filter(|(.., kind)| kind.0 == SHULKER)
        .map(|(shulker, layer, id, _)| (shulker, layer.0, id.0))
        .collect();
    for (shulker, layer, id) in shulkers {
        tend(world, shulker, layer, id, now);
    }
}
