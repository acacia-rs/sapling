//! A breeze's wind charges, as BDS's flew (research/captures/bds-breeze-findings.md): straight at
//! 0.7 a tick; one that meets a player stops and bursts 6 ticks later, hurting it 1, and one that
//! meets a block bursts there. A burst throws every player within 6 away from it.
// TODO: a charge is not shown hidden on its first tick; it hits no mob, pushes no mob, toggles no
// door or lever, flies on for ever when it meets nothing, and cannot be turned back.

use acacia_proto::packets::{LevelEvent, LevelEventEvent, LevelSoundEvent};
use acacia_proto::types::{MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1, Vec3f};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_data::AddEntityOptions;

use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Position, Tick, in_world};
use crate::sync::Told;
use crate::{entities, mobs, vitals};

const KIND: &str = "minecraft:breeze_wind_charge_projectile";
const SIZE: [f32; 2] = [0.3125, 0.3125];
const PLAYER: [f32; 2] = [0.6, 1.8];
/// It leaves this far up its breeze at this speed, aimed this far up its prey, each part of its
/// speed off by up to this; `shoot` is heard this far up.
const HELD: f32 = 1.416;
const SPEED: f32 = 0.7;
const AIMED: f32 = 0.9;
const SPREAD: f32 = 0.0075;
const SHOOT_UP: f32 = 0.885;
/// Ticks from meeting a player to bursting.
const HOLDS: u64 = 6;
const HARM: f32 = 1.0;
/// A burst reaches this far and throws this hard, at 0 off, towards a player's eyes.
const REACH: f32 = 6.0;
const PUSH: f32 = 0.6;
const EYES: f32 = 1.62;
/// It is drawn back this far from a block it meets.
const BACK: f32 = 0.25;

#[derive(Component)]
struct Charge {
    breeze: Entity,
    kind: String,
    /// The tick it met a player and whom, or the tick it burst and where.
    met: Option<(u64, Entity)>,
    burst: Option<[f32; 3]>,
}

fn heard(name: &str, [x, y, z]: [f32; 3], of: &str, id: i64) -> Bytes {
    sapling_net::encode(&LevelSoundEvent { sound_id: name.into(), position: Vec3f { x, y, z }, extra_data: -1, entity_type: of.into(), is_baby_mob: false, is_global: false, entity_unique_id: id, fire_at_position: None })
}

/// A breeze of `kind` with the body `breeze` shoots a charge at a player whose feet are `prey`.
pub(super) fn fire(world: &mut World, layer: Entity, breeze: Entity, kind: &str, body: &Body, prey: [f32; 3]) {
    let from = [body.at[0], body.at[1] + HELD, body.at[2]];
    let aim: [f32; 3] = std::array::from_fn(|axis| prey[axis] + if axis == 1 { AIMED } else { 0.0 } - from[axis]);
    let long = aim.iter().map(|c| c * c).sum::<f32>().sqrt().max(1e-4);
    let mut dice = world.get_resource_or_insert_with(Dice::default);
    let motion = aim.map(|c| c / long * SPEED + (dice.fraction() + dice.fraction() - 1.0) * SPREAD);
    let (yaw, pitch) = ((-motion[0]).atan2(motion[2]).to_degrees(), motion[1].atan2(motion[0].hypot(motion[2])).to_degrees());
    let owner = world.get::<NetId>(breeze).map_or(-1, |id| id.0 as i64);
    let announce = |id: u64| {
        let options = AddEntityOptions { id, identifier: KIND, feet: from, yaw, pitch, variant: 0, mark_variant: 0, color: 0, baby: false, flags: MetadataFlags1(0), health: None, name: "" };
        let mut shown = sapling_data::add_entity(&options);
        shown.velocity = Vec3f { x: motion[0], y: motion[1], z: motion[2] };
        shown.metadata.retain(|item| item.key != Key::OwnerEid);
        shown.metadata.push(MetadataDictionaryItem { key: Key::OwnerEid, r#type: Type::Long, legacy_type: 7, value: Value::Default(Plain::Long(owner)) });
        Some(vec![sapling_net::encode(&shown)])
    };
    let Some(charge) = entities::spawn(world, layer, KIND, Body::new(from, motion, SIZE), from, announce) else { return };
    world.entity_mut(charge).insert(Charge { breeze, kind: kind.to_owned(), met: None, burst: None });
    let up = [body.at[0], body.at[1] + SHOOT_UP, body.at[2]];
    entities::tell_viewers(world, breeze, layer, body.at.map(|c| c.floor() as i32), vec![heard("shoot", up, kind, owner), heard("undefined", up, "", -1)]);
    if let Some(id) = world.get::<NetId>(charge).map(|id| id.0) {
        fly(world, charge, layer, id);
    }
}

/// The player of `layer` whose box the charge's, its bottom middle at `at`, reaches into.
fn met(world: &World, layer: Entity, at: [f32; 3]) -> Option<Entity> {
    let across = (PLAYER[0] + SIZE[0]) / 2.0;
    in_world(world).into_iter().find(|player| {
        let Some(feet) = world.get::<Position>(*player).map(|position| position.feet) else { return false };
        world.get::<InLayer>(*player).map(|layer| layer.0) == Some(layer) && (at[0] - feet[0]).abs() < across && (at[2] - feet[2]).abs() < across && at[1] + SIZE[1] > feet[1] && at[1] < feet[1] + PLAYER[1]
    })
}

/// The burst at `at`: `struck` is hurt and thrown from the breeze, every other player within
/// [`REACH`] is pushed; seen and heard there.
fn burst(world: &mut World, charge: Entity, layer: Entity, at: [f32; 3], struck: Option<Entity>) -> Option<()> {
    let (breeze, kind) = world.get::<Charge>(charge).map(|charge| (charge.breeze, charge.kind.clone()))?;
    let from = world.get::<Body>(breeze).map_or(at, |body| body.at);
    let near: Vec<(Entity, [f32; 3])> = in_world(world).into_iter().filter(|player| world.get::<InLayer>(*player).map(|layer| layer.0) == Some(layer)).filter_map(|player| Some((player, world.get::<Position>(player)?.feet))).collect();
    for (player, feet) in near {
        let off = (0..3).map(|axis| (feet[axis] - at[axis]).powi(2)).sum::<f32>().sqrt();
        let towards: [f32; 3] = std::array::from_fn(|axis| feet[axis] + if axis == 1 { EYES } else { 0.0 } - at[axis]);
        let long = towards.iter().map(|c| c * c).sum::<f32>().sqrt().max(1e-4);
        let push = if off < REACH { towards.map(|c| c / long * PUSH * (1.0 - off / REACH)) } else { [0.0; 3] };
        match Some(player) == struck {
            true => drop(vitals::winded(world, player, HARM, &kind, from, push)),
            false if off < REACH => vitals::pushed(world, player, push),
            false => {}
        }
    }
    let [x, y, z] = at;
    let flash = LevelEvent { event: LevelEventEvent::ParticleBreezeWindExplosion, position: Vec3f { x, y, z }, data: 3 };
    let mut packets = vec![sapling_net::encode(&flash)];
    if struck.is_some() {
        packets.push(heard("undefined", at, "", -1));
    }
    entities::tell_viewers(world, charge, layer, at.map(|c| c.floor() as i32), packets);
    world.get_mut::<Charge>(charge)?.burst = Some(at);
    Some(())
}

/// A tick of one charge: on, stopped by a player, burst, or gone the tick after its burst.
fn fly(world: &mut World, charge: Entity, layer: Entity, id: u64) -> Option<()> {
    let now = world.resource::<Tick>().0;
    let mut body = *world.get::<Body>(charge)?;
    let (met_at, burst_at) = world.get::<Charge>(charge).map(|charge| (charge.met, charge.burst))?;
    if let Some(at) = burst_at {
        let sound = heard("breeze_wind_charge.burst", at, "", -1);
        entities::tell_viewers(world, charge, layer, at.map(|c| c.floor() as i32), vec![sound]);
        mobs::remove(world, charge);
        return None;
    }
    if let Some((since, player)) = met_at {
        if now >= since + HOLDS {
            burst(world, charge, layer, body.at, Some(player));
        }
        return Some(());
    }
    let to: [f32; 3] = std::array::from_fn(|axis| body.at[axis] + body.motion[axis]);
    let middle = [to[0], to[1] + SIZE[1] / 2.0, to[2]];
    let walled = !Body::new(to, [0.0; 3], SIZE).is_free(&world.get::<Layer>(layer)?.world);
    if walled {
        let long = body.motion.iter().map(|c| c * c).sum::<f32>().sqrt().max(1e-4);
        return burst(world, charge, layer, std::array::from_fn(|axis| middle[axis] - body.motion[axis] / long * BACK), None);
    }
    body.at = to;
    *world.get_mut::<Body>(charge)? = body;
    let packets = world.get_mut::<Told>(charge)?.update(id, body.at, body.motion, false);
    entities::tell_viewers(world, charge, layer, body.at.map(|c| c.floor() as i32), packets);
    if let Some(player) = met(world, layer, to) {
        world.get_mut::<Charge>(charge)?.met = Some((now, player));
    }
    Some(())
}

/// Moves every charge a tick.
pub(super) fn tick(world: &mut World) {
    let charges: Vec<(Entity, Entity, u64)> = world.query_filtered::<(Entity, &InLayer, &NetId), With<Charge>>().iter(world).map(|(charge, layer, id)| (charge, layer.0, id.0)).collect();
    for (charge, layer, id) in charges {
        fly(world, charge, layer, id);
    }
}
