//! What hurts a mob where it is: a fall, fire and lava, a block its head is in, water it cannot
//! breathe in, and powder snow (frost.rs), found each tick and told to its viewers as BDS tells them
//! (docs/research/mob-damage.md). The rules themselves: health.rs.
// Breath comes back at once out of water, as BDS's cow's 120 went to 300 (bds-n3breath.txt); an
// armour stand in fire is armour_stands/burning.rs's.

use bevy_ecs::prelude::*;
use bytes::Bytes;
use acacia_proto::packets::{EntityEvent, EntityEventEventId, LevelEvent, LevelEventEvent, LevelSoundEvent};
use acacia_proto::types::{MetadataFlags1, Vec3f};

use super::health::{Cause, Health, WALLED};
use crate::body::{Body, InLiquid, Liquid, in_liquid};
use crate::components::Layer;
use crate::sync::Told;
use crate::{entities, late_sounds, splash};

mod cells;

pub(crate) use cells::{holds, in_rain, is_full_block, touches};
pub(super) use cells::{is_rained_on, sun_on};
use cells::is_sunlit;

const WATER: &[&str] = &["minecraft:water", "minecraft:flowing_water"];
/// Kinds that rain and water hurt, 1 a hit as drowning: BDS's endermen in the rain
/// (research/captures/bds-kinds3.txt), blazes and snow golems in water and rain (bds-o3wet.txt).
// TODO: a wet enderman's teleports.
const WATER_HURTS: [&str; 3] = ["minecraft:enderman", "minecraft:blaze", "minecraft:snow_golem"];
/// A fall that costs more than this sounds big.
const BIG_FALL: f32 = 4.0;
/// A fall is heard the tick after it hurts.
const FALL_LATE: u64 = 1;
const BUBBLES: i32 = 8;
const DOUSED: i32 = -1;

/// Something costs a mob `cost` by `cause`, told to its viewers; it may die of it.
pub(crate) fn wound(world: &mut World, mob: Entity, layer: Entity, cost: f32, cause: Cause) -> Option<()> {
    wound_by(world, mob, layer, cost, cause, None).map(drop)
}

/// As [`wound`], by the player `by` if one: plugins are told who, and a mob it kills is the
/// player's, for its experience. Whether it took any of it.
pub(super) fn wound_by(world: &mut World, mob: Entity, layer: Entity, cost: f32, cause: Cause, by: Option<Entity>) -> Option<bool> {
    let (id, body, kind) = (world.get::<crate::components::NetId>(mob)?.0, *world.get::<Body>(mob)?, world.get::<entities::Kind>(mob)?.0.clone());
    let attacker = by.and_then(|player| crate::components::in_world_identity(world, player)).map(sapling_api::Attacker::Player);
    let cost = crate::damage::ask(world, mob, cost, cause, attacker)?;
    let mut health = *world.get::<Health>(mob)?;
    let took = health.hurt(cost);
    if took {
        *world.get_mut::<Health>(mob)? = health;
        entities::tell_viewers(world, mob, layer, body.at.map(|c| c.floor() as i32), vec![strike(id, &health, cause, None)]);
        by.inspect(|player| super::villagers::shot_standing(world, *player, mob, layer, &kind, body.at, health.left() <= 0.0));
        if health.left() <= 0.0 {
            if let Some(player) = by {
                super::raids::slain(world, player, mob);
                super::attacks::slay(world, mob, &kind, 0);
            }
            super::die(world, mob, layer, &kind, &body);
        }
    }
    Some(took)
}

/// A hit that costs nothing (a snowball on a cow): its viewers see it flinch, as BDS showed
/// (`HurtAnimation` 3, research/captures/bds-n3knock.txt).
pub(super) fn flinch(world: &mut World, mob: Entity, layer: Entity, cause: Cause) -> Option<()> {
    let (id, at, health) = (world.get::<crate::components::NetId>(mob)?.0, world.get::<Body>(mob)?.at, *world.get::<Health>(mob)?);
    entities::tell_viewers(world, mob, layer, at.map(|c| c.floor() as i32), vec![strike(id, &health, cause, None)]);
    Some(())
}

/// The sun burns a mob for this many ticks at a time.
const SUN_BURNS: u16 = 160;

/// The hit's flash, or the death it was; a fall's names where it landed.
pub(super) fn strike(id: u64, health: &Health, cause: Cause, landed: Option<[f32; 3]>) -> Bytes {
    strike_as(id, health, cause.id(), landed)
}

/// [`strike`] with the cause as the wire numbers it, for one the API's causes do not name.
pub(super) fn strike_as(id: u64, health: &Health, cause: i32, landed: Option<[f32; 3]>) -> Bytes {
    let (event_id, data) = if health.left() > 0.0 { (EntityEventEventId::HurtAnimation, cause) } else { (EntityEventEventId::DeathAnimation, 0) };
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
/// late, as on BDS. A `dying` mob is only told of. Each hit is offered to plugins as it lands.
pub(super) fn tick(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, start: &Body, now: &Body, dying: bool) -> bool {
    let Some(mut health) = world.get::<Health>(entity).copied() else { return false };
    let mut packets = Vec::new();
    if !dying {
        let traits = sapling_data::mob_traits(kind);
        let burns = !traits.fire_immune;
        let [x, y, z] = super::head(kind, start);
        let head = head_cell(kind, start);
        let (in_wall, under_water) = held_breath(world, entity, layer, kind, start);
        let (darken, rains) = (crate::light::darken(world, layer), crate::weather::rains(world, layer));
        let level = world.get::<Layer>(layer).map(|layer| &layer.world);
        // Fire and lava where the move left it, not where it began: BDS's chicken and pig over a
        // source were lit in the tick they first dipped into it and scalded in the one they sank
        // 0.4 in (bds-bodies.txt).
        let touching = if burns { level.and_then(|level| super::heat::fire_harm(level, now)) } else { None };
        let scorched = burns && now.on_ground && level.is_some_and(|level| super::heat::on_magma(level, now));
        let scalds = burns && level.is_some_and(|level| in_liquid(level, now, InLiquid::Sinks, Liquid::Lava));
        // Rain kept BDS's zombie from burning at noon (research/captures/bds-sun.txt) and put out a burning cow.
        let feet = start.at.map(|c| c.floor() as i32);
        let rained = level.is_some_and(|level| is_rained_on(level, feet, rains));
        let sunlit = traits.burns_in_sun && !rained && level.is_some_and(|level| is_sunlit(level, head, darken));
        let in_snow = level.is_some_and(|level| touches(level, now, super::frost::POWDER_SNOW));
        health.begin();
        let weigh = |world: &mut World, health: &mut Health, before: Health, cause: Cause| crate::damage::weigh(world, entity, health, &before, cause);
        let before = health;
        let ground = level.map_or(super::Ground::Soft(1.0), |level| super::heat::softening(level, now.at));
        let dropped = if now.on_ground && now.liquid().is_none() { health.dropped(start.at[1], now.at[1]) } else { 0.0 };
        super::treading::tread(world, layer, now, dropped);
        if let Some((harm, from, cause)) = health.fall_onto(start.at, now.at[1], now.on_ground, now.liquid().is_some() || traits.no_fall, ground)
            && weigh(world, &mut health, before, cause)
        {
            packets.push(strike(id, &health, cause, (cause == Cause::Fall).then_some(now.at)));
            packets.push(land(world, layer, id, kind, from, now, harm));
        }
        let wet = now.liquid() == Some(Liquid::Water);
        if burns && !wet && sunlit {
            health.kindle(SUN_BURNS);
        }
        let (lit, before) = (health.is_burning(), health);
        if rained {
            health.rained_on();
        }
        let fire = if burns { health.burn(touching, wet) } else { None };
        let fire = fire.filter(|cause| weigh(world, &mut health, before, *cause));
        // Rain put BDS's cow out with the same steam and fizz as water (research/captures/bds-rainvoid.txt).
        if lit && !health.is_burning() && (wet || rained) {
            packets.push(steam(now));
            splash::fizz(world, layer, now, DOUSED);
        }
        packets.extend(fire.map(|cause| strike(id, &health, cause, None)));
        let before = health;
        if scorched && health.scorch() && weigh(world, &mut health, before, Cause::Magma) {
            packets.push(strike(id, &health, Cause::Magma, None));
        }
        let before = health;
        if WATER_HURTS.contains(&kind) && (wet || rained) && health.hurt(health.warded(1.0, Cause::Drowning)) && weigh(world, &mut health, before, Cause::Drowning) {
            packets.push(strike(id, &health, Cause::Drowning, None));
        }
        let before = health;
        if health.smother(in_wall, WALLED) && weigh(world, &mut health, before, Cause::Suffocation) {
            packets.push(strike(id, &health, Cause::Suffocation, None));
        }
        tell(world, entity, &mut health, id, kind, burns, &mut packets);
        let (frozen, freezes) = super::frost::chill(world, entity, id, kind, in_snow);
        packets.extend(frozen);
        let before = health;
        if let Some(harm) = freezes
            && health.hurt(health.warded(harm, Cause::Freezing))
            && weigh(world, &mut health, before, Cause::Freezing)
        {
            packets.push(strike(id, &health, Cause::Freezing, None));
        }
        let before = health;
        if scalds && health.scald() && weigh(world, &mut health, before, Cause::Lava) {
            packets.push(strike(id, &health, Cause::Lava, None));
        }
        let dry = traits.no_air && now.liquid() != Some(Liquid::Water);
        let held = in_wall || under_water || dry;
        let breath = health.breathe(held);
        let before = health;
        if health.drown(under_water) && weigh(world, &mut health, before, Cause::Drowning) {
            packets.push(strike(id, &health, Cause::Drowning, None));
        }
        let before = health;
        if health.gasp(dry) && weigh(world, &mut health, before, Cause::Suffocation) {
            packets.push(strike(id, &health, Cause::Suffocation, None));
        }
        let before = health;
        if super::dolphins::dried(world, entity, wet || rained) && health.hurt(1.0) && weigh(world, &mut health, before, Cause::Generic) {
            packets.push(strike(id, &health, Cause::Generic, None));
        }
        packets.extend(breath_told(world, entity, id, held, breath));
        if under_water {
            packets.push(sapling_net::encode(&LevelEvent { event: LevelEventEvent::ParticleBubble, position: Vec3f { x, y, z }, data: BUBBLES }));
        }
    } else {
        // BDS puts a burning body out while it lies dead.
        if health.is_burning() && now.liquid() == Some(Liquid::Water) {
            health.burn(None, true);
            packets.push(steam(now));
            splash::fizz(world, layer, now, DOUSED);
        }
        tell(world, entity, &mut health, id, kind, true, &mut packets);
    }
    if let Some(mut kept) = world.get_mut::<Health>(entity) {
        *kept = health;
    }
    entities::tell_viewers(world, entity, layer, now.at.map(|c| c.floor() as i32), packets);
    !dying && health.left() <= 0.0
}

/// The cell of a mob's head: straight above its feet, not forward as the head is drawn.
fn head_cell(kind: &str, body: &Body) -> [i32; 3] {
    [body.at[0], super::head(kind, body)[1], body.at[2]].map(|c| c.floor() as i32)
}

/// Whether the mob at `body` holds its breath for its head in a block, and for its head in water
/// it cannot breathe.
fn held_breath(world: &World, entity: Entity, layer: Entity, kind: &str, body: &Body) -> (bool, bool) {
    let traits = sapling_data::mob_traits(kind);
    let breathes = traits.breathes_water || traits.breath.is_some() || super::afflictions::level(world, entity, sapling_api::Effect::WaterBreathing).is_some();
    let head = head_cell(kind, body);
    world.get::<Layer>(layer).map_or((false, false), |layer| (is_full_block(&layer.world, head), !breathes && holds(&layer.world, head, WATER)))
}

/// What tells viewers that the mob `id` started or stopped breathing (`held`), with its `breath`
/// where that changed.
fn breath_told(world: &mut World, entity: Entity, id: u64, held: bool, breath: Option<i16>) -> Option<Bytes> {
    let breathing = world.get_mut::<Told>(entity).and_then(|mut told| told.change_flag(MetadataFlags1::BREATHING, !held));
    sapling_data::mob_breath(id, breathing, breath).map(|told| sapling_net::encode(&told))
}

/// The breath a mob takes in the tick it spawns, as BDS's does: its viewers see it breathing, or
/// holding its breath where it spawned with its head in a block or water (research/captures/bds-kill.txt).
pub(super) fn first_breath(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body) {
    let (in_wall, under_water) = held_breath(world, entity, layer, kind, body);
    let held = in_wall || under_water;
    let Some(breath) = world.get_mut::<Health>(entity).map(|mut health| health.breathe(held)) else { return };
    let told = breath_told(world, entity, id, held, breath);
    entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), told.into_iter().collect());
}

/// Tells viewers the mob's health, and its catching or losing fire, where they changed.
fn tell(world: &mut World, entity: Entity, health: &mut Health, id: u64, kind: &str, burns: bool, packets: &mut Vec<Bytes>) {
    if health.left() != health.told {
        health.told = health.left();
        packets.push(sapling_net::encode(&super::boosts::told(world, entity, id, health)));
    }
    if burns && health.is_burning() != health.lit {
        health.lit = health.is_burning();
        let flags = entities::mob_flags(world, entity, kind, MetadataFlags1::ONFIRE, health.lit);
        packets.push(sapling_net::encode(&sapling_data::mob_flags_set(id, flags)));
    }
}
