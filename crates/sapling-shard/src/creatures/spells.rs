//! What an evoker casts at its prey, as BDS's did (research/captures/bds-evoker-findings.md): it
//! stands for the cast, and a second in sends a line of sixteen fangs at a prey 3 or more off, or
//! two rings of them about itself at one nearer. A fang bites a tick after it rises.
// TODO: no vexes (they fly), no stepping away from a player within 8, and a fang bites no mob.

use acacia_proto::packets::{EntityEvent, EntityEventEventId};
use acacia_proto::types::{MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1};
use bevy_ecs::prelude::*;
use sapling_data::AddEntityOptions;
use sapling_world::World as Level;

use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Position, Tick, in_world};
use super::{hunting, legs};
use crate::{entities, late_sounds, mobs, vitals};

/// A cast shows for this many ticks, its fangs come this many after it began, and the next
/// begins this many after it ends.
const CASTS: u64 = 39;
const FIRST: u64 = 21;
const RESTS: u64 = 101;
/// Nearer than this its prey gets the rings, each of so many fangs this far out, so many ticks
/// apart; farther, the line, a fang a tick, each this much farther out.
const NEAR: f32 = 3.0;
const RINGS: [(usize, f32); 2] = [(5, 1.5), (8, 2.5)];
const RING_GAP: u64 = 3;
const LINE: u64 = 16;
const STEP: f32 = 1.25;
const COLOUR: i32 = 0xFF66_4D59_u32 as i32;
const FANG: &str = "minecraft:evocation_fang";
const SIZE: [f32; 2] = [1.0, 0.8];
const PLAYER: [f32; 2] = [0.6, 1.8];
/// A fang's `LimitedLife` as it is announced, and the ticks it lasts.
const LIFE: i32 = 22;
const LASTS: u64 = 11;
/// A fang costs this, whatever the difficulty, and its `throw` is heard this much of its height up it.
const HARM: f32 = 6.0;
const THROW_UP: f32 = 0.5;

#[derive(Component, Default)]
pub(super) struct Caster {
    next: u64,
    cast: Option<Cast>,
}

#[derive(Clone, Copy)]
struct Cast {
    began: u64,
    rings: bool,
    /// Taken as the fangs begin: the way to the prey and the heights a fang may stand between.
    aim: Option<([f32; 2], [f32; 2])>,
}

#[derive(Component)]
struct Fang {
    owner: [f32; 3],
    kind: String,
    born: u64,
}

pub(super) fn arm(world: &mut World, entity: Entity, kind: &str) {
    if sapling_data::mob_traits(kind).hunts.is_some_and(|hunts| hunts.fights == Some("spells")) {
        world.entity_mut(entity).insert(Caster::default());
    }
}

fn limited_life(id: u64, left: i32) -> bytes::Bytes {
    let item = MetadataDictionaryItem { key: Key::LimitedLife, r#type: Type::Int, legacy_type: 2, value: Value::Default(Plain::Int(left)) };
    sapling_net::encode(&acacia_proto::packets::SetEntityData { runtime_entity_id: id, metadata: vec![item], properties: acacia_proto::types::EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 })
}

/// Before the caster's move, with its prey at `prey`: whether it stands to cast. One that lost
/// its prey ends its cast, as BDS's did after 5 of 16 fangs.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body, prey: Option<[f32; 3]>) -> bool {
    let now = world.resource::<Tick>().0;
    let Some(caster) = world.get::<Caster>(entity) else { return false };
    let (next, cast) = (caster.next, caster.cast);
    let cell = body.at.map(|c| c.floor() as i32);
    match (cast, prey) {
        (Some(_), None) => end(world, entity, layer, id, kind, cell, now),
        (Some(cast), Some(feet)) => {
            conjure(world, entity, layer, id, kind, body, cast, feet, now);
            if now >= cast.began + CASTS {
                end(world, entity, layer, id, kind, cell, now);
            }
            legs::rest(world, entity);
            return true;
        }
        (None, Some(feet)) if now >= next => {
            let rings = (feet[0] - body.at[0]).hypot(feet[2] - body.at[2]) < NEAR;
            world.get_mut::<Caster>(entity).expect("a caster, above").cast = Some(Cast { began: now, rings, aim: None });
            entities::tell_viewers(world, entity, layer, cell, vec![sapling_net::encode(&sapling_data::mob_spell(id, kind, Some(COLOUR)))]);
            late_sounds::play_later(world, 1, layer, cell, hunting::sound("cast.spell", id, kind, body, 0.0));
            if rings {
                late_sounds::play_later(world, 1, layer, cell, hunting::sound("prepare.attack", id, kind, body, 0.0));
            }
            legs::rest(world, entity);
            return true;
        }
        _ => {}
    }
    false
}

fn end(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, cell: [i32; 3], now: u64) {
    if let Some(mut caster) = world.get_mut::<Caster>(entity) {
        (caster.cast, caster.next) = (None, now + RESTS);
    }
    entities::tell_viewers(world, entity, layer, cell, vec![sapling_net::encode(&sapling_data::mob_spell(id, kind, None))]);
}

/// The fangs of this tick of the cast.
fn conjure(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body, mut cast: Cast, feet: [f32; 3], now: u64) {
    let Some(step) = now.checked_sub(cast.began + FIRST) else { return };
    let ([dx, dz], heights) = *cast.aim.get_or_insert_with(|| {
        let (dx, dz) = (feet[0] - body.at[0], feet[2] - body.at[2]);
        let far = dx.hypot(dz).max(1e-4);
        ([dx / far, dz / far], [body.at[1].min(feet[1]), body.at[1].max(feet[1])])
    });
    if let Some(mut caster) = world.get_mut::<Caster>(entity) {
        caster.cast = Some(cast);
    }
    let spots: Vec<[f32; 2]> = match cast.rings {
        true => {
            let Some(&(count, radius)) = RINGS.get((step / RING_GAP) as usize).filter(|_| step.is_multiple_of(RING_GAP)) else { return };
            if step == 0 {
                late_sounds::play_later(world, 0, layer, body.at.map(|c| c.floor() as i32), hunting::sound("prepare.attack", id, kind, body, 0.0));
            }
            let toward = dz.atan2(dx);
            (0..count).map(|i| toward + i as f32 * std::f32::consts::TAU / count as f32).map(|angle| [body.at[0] + radius * angle.cos(), body.at[2] + radius * angle.sin()]).collect()
        }
        false if step < LINE => vec![[body.at[0] + dx * STEP * (step + 1) as f32, body.at[2] + dz * STEP * (step + 1) as f32]],
        false => return,
    };
    for [x, z] in spots {
        raise(world, layer, entity, kind, body.at, [x.floor() as i32, z.floor() as i32], heights);
    }
}

/// The first cell from just above the higher of `heights` down to just below the lower that
/// stands on a full block and is not one.
fn ground(level: &Level, [x, z]: [i32; 2], [low, high]: [f32; 2]) -> Option<i32> {
    let full = |y: i32| level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_full_cube());
    let loaded = level.is_loaded(sapling_world::ChunkPos::of_block(x, z));
    (low.floor() as i32 - 1..=high.floor() as i32 + 1).rev().find(|&y| loaded && full(y - 1) && !full(y))
}

/// A fang of the caster `owner` rises at the column `[x, z]`, if it has ground there.
fn raise(world: &mut World, layer: Entity, owner: Entity, kind: &str, from: [f32; 3], [x, z]: [i32; 2], heights: [f32; 2]) -> Option<()> {
    let y = ground(&world.get::<Layer>(layer)?.world, [x, z], heights)?;
    let at = [x as f32, y as f32, z as f32];
    let owner_id = world.get::<NetId>(owner)?.0 as i64;
    let announce = |id: u64| {
        let options = AddEntityOptions { id, identifier: FANG, feet: at, yaw: 0.0, pitch: 0.0, variant: 0, mark_variant: 0, color: 0, baby: false, flags: MetadataFlags1(0), health: None, name: "" };
        let mut shown = sapling_data::add_entity(&options);
        shown.metadata.retain(|item| item.key != Key::OwnerEid && item.key != Key::LimitedLife);
        shown.metadata.push(MetadataDictionaryItem { key: Key::OwnerEid, r#type: Type::Long, legacy_type: 7, value: Value::Default(Plain::Long(owner_id)) });
        shown.metadata.push(MetadataDictionaryItem { key: Key::LimitedLife, r#type: Type::Int, legacy_type: 2, value: Value::Default(Plain::Int(LIFE)) });
        Some(vec![sapling_net::encode(&shown)])
    };
    let fang = entities::spawn(world, layer, FANG, Body::new(at, [0.0; 3], SIZE), at, announce)?;
    let born = world.resource::<Tick>().0;
    world.entity_mut(fang).insert(Fang { owner: from, kind: kind.to_owned(), born });
    let id = world.get::<NetId>(fang)?.0;
    entities::tell_viewers(world, fang, layer, [x, y, z], vec![limited_life(id, LIFE - 2)]);
    Some(())
}

/// The players of `layer` a fang standing at `at` reaches.
fn bitten(world: &World, layer: Entity, at: [f32; 3]) -> Vec<Entity> {
    let across = (SIZE[0] + PLAYER[0]) / 2.0;
    in_world(world).into_iter().filter(|player| world.get::<InLayer>(*player).map(|layer| layer.0) == Some(layer)).filter(|player| {
        world.get::<Position>(*player).is_some_and(|position| {
            let feet = position.feet;
            (feet[0] - at[0]).abs() < across && (feet[2] - at[2]).abs() < across && feet[1] < at[1] + SIZE[1] && at[1] < feet[1] + PLAYER[1]
        })
    }).collect()
}

/// A tick of every fang: it bites on its second, is heard on its third, wanes and goes.
pub(super) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let fangs: Vec<(Entity, Entity, u64, [f32; 3], u64)> =
        world.query::<(Entity, &InLayer, &NetId, &Body, &Fang)>().iter(world).map(|(fang, layer, id, body, state)| (fang, layer.0, id.0, body.at, now - state.born)).collect();
    for (fang, layer, id, at, age) in fangs {
        if age >= LASTS {
            mobs::remove(world, fang);
            continue;
        }
        let cell = at.map(|c| c.floor() as i32);
        let mut packets = Vec::new();
        if age == 1 {
            packets.push(sapling_net::encode(&EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::ArmSwing, data: 0, fire_at_position: None }));
            let Some((owner, kind)) = world.get::<Fang>(fang).map(|fang| (fang.owner, fang.kind.clone())) else { continue };
            for player in bitten(world, layer, at) {
                vitals::fanged(world, player, HARM, &kind, owner);
            }
        }
        if age == 2 {
            let body = Body::new(at, [0.0; 3], SIZE);
            packets.extend([hunting::sound("throw", id, FANG, &body, THROW_UP), hunting::sound("fang", id, FANG, &body, 0.0)]);
        }
        // 20 on its first tick, 2 less each after, and 1 on its last.
        packets.push(limited_life(id, if age == LASTS - 1 { 1 } else { LIFE - 2 - 2 * age as i32 }));
        entities::tell_viewers(world, fang, layer, cell, packets);
    }
}
