//! How a mob is shown: its announcement, as BDS announced its kind or as it was stored, where its
//! head is, the sound it makes as it appears, and how its body falls and slows.

use acacia_proto::packets::LevelSoundEvent;
use acacia_proto::types::{MetadataFlags1, Vec3f};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_data::AddEntityOptions;
use sapling_world::Actor;

use super::Creature;
use super::genes::Gene;
use super::health::Health;
use super::husbandry::Raised;
use super::legs::Legs;
use super::vexes;
use crate::body::{Body, Physics};
use crate::components::NetId;
use crate::sync::Told;
use crate::{entities, mobs};

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
/// announced its kind, in `variant`; either as it is `raised` now.
// TODO: a mob dressed as it was stored is not shown on fire to one who comes to see it burning.
pub(super) fn announcement(id: u64, kind: &str, body: &Body, (look, raised): (Option<&Actor>, Option<&Raised>), health: Option<&Health>, yaw: f32, variant: i32) -> Vec<Bytes> {
    let left = health.map(Health::left);
    let extra = raised.map_or(MetadataFlags1(0), Raised::unstored_flags);
    let dressed = look.map(|look| raised.map_or_else(|| look.clone(), |raised| raised.dress(look)));
    if let Some(shown) = dressed.and_then(|look| mobs::announcement_with(&Actor { position: body.at, health: left.or(look.health), ..look }, id, extra)) {
        return shown;
    }
    let on_fire = health.is_some_and(Health::is_burning);
    let fire = if on_fire { MetadataFlags1::ONFIRE.0 } else { 0 };
    let kept = raised.map(|raised| &raised.kept);
    let options = AddEntityOptions {
        id,
        identifier: kind,
        feet: body.at,
        yaw,
        pitch: 0.0,
        variant,
        mark_variant: kept.map_or(0, |kept| kept.mark_variant),
        color: kept.map_or(0, |kept| kept.colour),
        baby: raised.is_some_and(Raised::is_baby),
        flags: MetadataFlags1(fire | raised.map_or(0, |raised| raised.flags().0)),
        health: left,
        name: kept.map_or("", |kept| kept.name.as_str()),
    };
    let mut added = sapling_data::add_entity(&options);
    super::slimes::sizes::dress(&mut added, kind, Some(super::genes::Gene(variant)));
    let mut shown = vec![sapling_net::encode(&added)];
    let held = HOLDS.iter().find(|(of, _)| *of == kind).map(|(_, item)| sapling_data::Stack { name: item, count: 1, damage: 0, block: None, tag: None });
    shown.extend(held.and_then(|stack| sapling_data::mob_hand(id, &stack, false)).map(|held| sapling_net::encode(&held)));
    shown
}

/// What tells a viewer of the mob now: dressed as stored and raised, its health and facing.
fn shown_now(world: &World, entity: Entity) -> Option<Vec<Bytes>> {
    let (id, kind, body) = (world.get::<NetId>(entity)?.0, &world.get::<entities::Kind>(entity)?.0, world.get::<Body>(entity)?);
    let raised = world.get::<Raised>(entity);
    let look = world.get::<Creature>(entity).and_then(|creature| creature.look.as_ref());
    let yaw = world.get::<Legs>(entity).map_or(0.0, |legs| legs.yaw);
    let mut shown = announcement(id, kind, body, (look, raised), world.get::<Health>(entity), yaw, world.get::<Gene>(entity).map_or(0, |gene| gene.0));
    let flags = world.get::<Told>(entity).and_then(Told::mob_flags);
    shown.extend(flags.and_then(|flags| super::husbandry::links_packet(world, entity, id, flags)));
    Some(shown)
}

/// Players who come to see the mob later are told of it as it is now.
pub(super) fn reshow(world: &mut World, entity: Entity) {
    if let Some(shown) = shown_now(world, entity) {
        mobs::reannounce(world, entity, shown);
    }
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

/// How a mob of `kind` falls and slows: as most mobs, but for those that leap, fly and swim.
pub(super) fn physics_of(kind: &str) -> &'static Physics {
    match kind {
        "minecraft:breeze" => &Physics::BREEZE,
        "minecraft:ghast" | super::wither::KIND => &Physics::FLYER,
        vexes::KIND => &Physics::GHOST,
        "minecraft:dolphin" => &Physics::SWIMMER,
        _ => &Physics::MOB,
    }
}
