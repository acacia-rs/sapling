//! What hurts a mob where it is: a fall, fire and lava, a block its head is in, and water it
//! cannot breathe in, found each tick and told to its viewers as BDS tells them
//! (docs/research/mob-damage.md). The rules themselves: health.rs.
// TODO: any full block but glass and leaves smothers (glass drilled, bds-smother.txt); a fish out of water breathes;
// breath comes back at once; an armour stand has no health here, though BDS burns it.

use bevy_ecs::prelude::*;
use bytes::Bytes;
use acacia_proto::packets::{EntityEvent, EntityEventEventId, LevelEvent, LevelEventEvent, LevelSoundEvent};
use acacia_proto::types::Vec3f;
use sapling_world::World as Level;

use super::health::{Cause, Health, WALLED};
use crate::body::{Body, Liquid};
use crate::components::Layer;
use crate::{entities, late_sounds, splash};

const FIERY: &[&str] = &["minecraft:lava", "minecraft:flowing_lava", "minecraft:fire", "minecraft:soul_fire"];
const WATER: &[&str] = &["minecraft:water", "minecraft:flowing_water"];
/// A fall that costs more than this sounds big.
const BIG_FALL: f32 = 4.0;
/// A fall is heard the tick after it hurts.
const FALL_LATE: u64 = 1;
const BUBBLES: i32 = 8;
const DOUSED: i32 = -1;

/// Whether the body's box reaches into a cell holding one of `names`, however full.
pub(crate) fn touches(level: &Level, body: &Body, names: &[&str]) -> bool {
    const EDGE: f32 = 1e-4;
    let (low, high) = body.bounds();
    let cells = |axis: usize| low[axis].floor() as i32..=(high[axis] - EDGE).floor() as i32;
    cells(0).any(|x| cells(1).any(|y| cells(2).any(|z| holds(level, [x, y, z], names))))
}

/// Whether the cell holds one of `names`, as a block or as the liquid in one.
pub(crate) fn holds(level: &Level, [x, y, z]: [i32; 3], names: &[&str]) -> bool {
    let registry = level.registry();
    is_within(level, [x, y, z]) && [level.block(x, y, z), level.liquid(x, y, z)].into_iter().any(|id| registry.get(id).is_some_and(|state| names.contains(&state.name)))
}

fn is_within(level: &Level, [x, y, z]: [i32; 3]) -> bool {
    level.in_height(y) && level.is_loaded(sapling_world::ChunkPos::of_block(x, z))
}

pub(crate) fn is_full_block(level: &Level, [x, y, z]: [i32; 3]) -> bool {
    let smothers = |name: &str| !name.ends_with("glass") && !name.ends_with("leaves");
    is_within(level, [x, y, z]) && level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_full_cube() && smothers(state.name))
}

/// The hit's flash, or the death it was; a fall's names where it landed.
pub(super) fn strike(id: u64, health: &Health, cause: Cause, landed: Option<[f32; 3]>) -> Bytes {
    let (event_id, data) = if health.left() > 0.0 { (EntityEventEventId::HurtAnimation, cause as i32) } else { (EntityEventEventId::DeathAnimation, 0) };
    let fire_at_position = landed.filter(|_| health.left() > 0.0).map(|[x, y, z]| Vec3f { x, y, z });
    sapling_net::encode(&EntityEvent { runtime_entity_id: id, event_id, data, fire_at_position })
}

/// The dust of a landing that cost `harm`, and its two sounds a tick later: `from` where the tick
/// of the landing began, the second naming the block landed on.
fn land(world: &mut World, layer: Entity, id: u64, kind: &str, from: [f32; 3], now: &Body, harm: f32) -> Bytes {
    let cell = now.at.map(|c| c.floor() as i32);
    let under = world.get::<Layer>(layer).map_or(0, |layer| layer.world.registry().network_hash(layer.world.block(cell[0], (now.at[1] - 0.5).floor() as i32, cell[2])));
    let [x, y, z] = from;
    let sound = |sound_id: &str, extra_data: i32| {
        sapling_net::encode(&LevelSoundEvent { sound_id: sound_id.into(), position: Vec3f { x, y, z }, extra_data, entity_type: kind.into(), is_baby_mob: false, is_global: false, entity_unique_id: id as i64, fire_at_position: None })
    };
    for sound in [sound(if harm > BIG_FALL { "fall.big" } else { "fall.small" }, -1), sound("fall", under as i32)] {
        late_sounds::play_later(world, FALL_LATE, layer, cell, sound);
    }
    sapling_net::encode(&EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::DustParticles, data: 0, fire_at_position: None })
}

/// The steam of a burning body put out by water: from the low corner of its box, its data the
/// box's width and height in blocks begun (research/captures/bds-douse.txt).
fn steam(body: &Body) -> Bytes {
    let [width, height] = body.size;
    let [x, y, z] = body.at;
    let position = Vec3f { x: x - width / 2.0, y, z: z - width / 2.0 };
    sapling_net::encode(&LevelEvent { event: LevelEventEvent::ParticleFizzEffect, position, data: width.ceil() as i32 | (height.ceil() as i32) << 8 })
}

/// A tick of the mob `id`'s health, its body `start` before the tick's move and `now` after;
/// whether it died of it, which its viewers are then told here. They are told its health after a
/// fall, fire and a block have hurt it and before lava and water do, so those two show a tick
/// late, as on BDS. A `dying` mob is only told of.
pub(super) fn tick(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, start: &Body, now: &Body, dying: bool) -> bool {
    let Some(mut health) = world.get::<Health>(entity).copied() else { return false };
    let mut packets = Vec::new();
    if !dying {
        let traits = sapling_data::mob_traits(kind);
        let burns = !traits.fire_immune;
        let [x, y, z] = super::head(kind, start);
        // Straight above its feet, not forward as the head is drawn.
        let head = [start.at[0], y, start.at[2]].map(|c| c.floor() as i32);
        let level = world.get::<Layer>(layer).map(|layer| &layer.world);
        let (touching, in_wall, under_water) = level.map_or((false, false, false), |level| (burns && touches(level, start, FIERY), is_full_block(level, head), !traits.breathes_water && holds(level, head, WATER)));
        health.begin();
        if let Some((harm, from)) = health.fall(start.at, now.at[1], now.on_ground, now.liquid().is_some() || traits.no_fall) {
            packets.push(strike(id, &health, Cause::Fall, Some(now.at)));
            packets.push(land(world, layer, id, kind, from, now, harm));
        }
        let wet = now.liquid() == Some(Liquid::Water);
        let lit = health.is_burning();
        let fire = if burns { health.burn(touching, wet) } else { None };
        if lit && !health.is_burning() && wet {
            packets.push(steam(now));
            splash::fizz(world, layer, now, DOUSED);
        }
        packets.extend(fire.map(|cause| strike(id, &health, cause, None)));
        if health.smother(in_wall, WALLED) {
            packets.push(strike(id, &health, Cause::Wall, None));
        }
        tell(&mut health, id, kind, burns, &mut packets);
        if burns && now.liquid() == Some(Liquid::Lava) && health.scald() {
            packets.push(strike(id, &health, Cause::Lava, None));
        }
        let breath = health.breathe(in_wall || under_water);
        if health.drown(under_water) {
            packets.push(strike(id, &health, Cause::Drowning, None));
        }
        packets.extend(breath.map(|air| sapling_net::encode(&sapling_data::mob_air(id, air))));
        if under_water {
            packets.push(sapling_net::encode(&LevelEvent { event: LevelEventEvent::ParticleBubble, position: Vec3f { x, y, z }, data: BUBBLES }));
        }
    } else {
        // BDS puts a burning body out while it lies dead.
        if health.is_burning() && now.liquid() == Some(Liquid::Water) {
            health.burn(false, true);
            packets.push(steam(now));
            splash::fizz(world, layer, now, DOUSED);
        }
        tell(&mut health, id, kind, true, &mut packets);
    }
    if let Some(mut kept) = world.get_mut::<Health>(entity) {
        *kept = health;
    }
    entities::tell_viewers(world, entity, layer, now.at.map(|c| c.floor() as i32), packets);
    !dying && health.left() <= 0.0
}

/// Tells viewers the mob's health, and its catching or losing fire, where they changed.
fn tell(health: &mut Health, id: u64, kind: &str, burns: bool, packets: &mut Vec<Bytes>) {
    if health.left() != health.told {
        health.told = health.left();
        packets.push(sapling_net::encode(&sapling_data::mob_health_update(id, health.left(), health.max())));
    }
    if burns && health.is_burning() != health.lit {
        health.lit = health.is_burning();
        packets.push(sapling_net::encode(&sapling_data::mob_fire_flags(id, kind, health.lit)));
    }
}
