//! How a mob is shown: its announcement, as BDS announced its kind or as it was stored, where its
//! head is, the sound it makes as it appears, and how its body falls and slows.

use acacia_proto::packets::LevelSoundEvent;
use acacia_proto::types::{MetadataFlags1, Vec3f};
use bytes::Bytes;
use sapling_data::AddEntityOptions;
use bevy_ecs::prelude::*;
use sapling_world::{Actor, ActorState};

use super::health::Health;
use super::vexes;
use crate::body::{Body, Physics};
use crate::mobs;

/// What a spawned mob of each of these kinds holds, as BDS showed it (bds-evoker-findings.md).
const HOLDS: &[(&str, &str)] = &[(vexes::KIND, "minecraft:iron_sword")];

/// Kinds whose head sits forward of their feet, by this much of their width (BDS's `Animal`
/// constructors); every other mob's is straight up (docs/research/spawn-sound.md).
const ANIMALS: &[&str] = &[
    "minecraft:chicken", "minecraft:cow", "minecraft:mooshroom", "minecraft:rabbit", "minecraft:ocelot", "minecraft:horse", "minecraft:llama", "minecraft:turtle", "minecraft:cat",
    "minecraft:pig",
];
const HEAD_FORWARD: f32 = 0.85;
/// A mob's head is this much of its height up.
const HEAD_UP: f32 = 0.85;

/// The mob `id` as its viewers are told of it: dressed as it was stored (`look`), or as BDS
/// announced its kind, in `variant`.
// TODO: a mob dressed as it was stored is not shown on fire to one who comes to see it burning.
pub(super) fn announcement(id: u64, kind: &str, body: &Body, look: Option<&Actor>, health: Option<&Health>, yaw: f32, variant: i32) -> Vec<Bytes> {
    let left = health.map(Health::left);
    if let Some(shown) = look.and_then(|look| mobs::announcement(&Actor { position: body.at, health: left.or(look.health), ..look.clone() }, id)) {
        return shown;
    }
    let on_fire = health.is_some_and(Health::is_burning);
    let options = AddEntityOptions {
        id,
        identifier: kind,
        feet: body.at,
        yaw,
        pitch: 0.0,
        variant,
        mark_variant: 0,
        color: 0,
        baby: false,
        flags: MetadataFlags1(if on_fire { MetadataFlags1::ONFIRE.0 } else { 0 }),
        health: left,
        name: "",
    };
    let mut added = sapling_data::add_entity(&options);
    super::slimes::sizes::dress(&mut added, kind, Some(super::genes::Gene(variant)));
    let mut shown = vec![sapling_net::encode(&added)];
    let held = HOLDS.iter().find(|(of, _)| *of == kind).map(|(_, item)| sapling_data::Stack { name: item, count: 1, damage: 0, block: None, tag: None });
    shown.extend(held.and_then(|stack| sapling_data::mob_hand(id, &stack, false)).map(|held| sapling_net::encode(&held)));
    shown
}

/// Where a mob's head is, as BDS places it; it faces +z (yaw 0).
pub(super) fn head(kind: &str, body: &Body) -> [f32; 3] {
    let ([x, y, z], [width, height]) = (body.at, body.size);
    let forward = if ANIMALS.contains(&kind) { HEAD_FORWARD * width } else { 0.0 };
    [x, y + HEAD_UP * height, z + forward]
}

/// The sound `name` a mob makes as it appears, at its head: `spawn`, or a split slime's `born`.
pub(super) fn spawn_sound(name: &str, id: u64, kind: &str, body: &Body) -> Bytes {
    let [x, y, z] = head(kind, body);
    let position = Vec3f { x, y, z };
    let sound = LevelSoundEvent { sound_id: name.into(), position, extra_data: -1, entity_type: kind.into(), is_baby_mob: false, is_global: false, entity_unique_id: id as i64, fire_at_position: None };
    sapling_net::encode(&sound)
}

/// Whether a mob was stored with `state` (saddled, tamed); a spawned one has none.
pub(crate) fn was_stored(world: &World, entity: Entity, state: ActorState) -> bool {
    world.get::<super::Creature>(entity).and_then(|creature| creature.look.as_ref()).is_some_and(|look| look.states.contains(&state))
}

pub(crate) fn is_baby(world: &World, entity: Entity) -> bool {
    world.get::<super::Creature>(entity).is_some_and(super::Creature::is_baby)
}

/// How a mob of `kind` falls and slows: as most mobs, but for those that leap, fly and swim.
pub(super) fn physics_of(kind: &str) -> &'static Physics {
    match kind {
        "minecraft:breeze" => &Physics::BREEZE,
        "minecraft:ghast" => &Physics::FLYER,
        vexes::KIND => &Physics::GHOST,
        "minecraft:dolphin" => &Physics::SWIMMER,
        _ => &Physics::MOB,
    }
}
