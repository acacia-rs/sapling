//! How a mob is shown: its announcement, as BDS announced its kind or as it was stored, where its
//! head is, the sound it makes as it appears, and how its body falls and slows.

use acacia_proto::packets::LevelSoundEvent;
use acacia_proto::types::{MetadataFlags1, Vec3f};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_data::{AddEntityOptions, Strides};
use sapling_world::Actor;

use super::Creature;
use super::genes::Gene;
use super::health::Health;
use super::husbandry::Raised;
use super::legs::Legs;
use super::properties::Properties;
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

/// What a mob is dressed in beyond its kind: as it was stored, raised, its entity properties,
/// a mount's strides and a llama's strength.
pub(super) type Dress<'a> = (Option<&'a Actor>, Option<&'a Raised>, Option<&'a Properties>, Option<&'a Strides>, Option<i32>);

/// The mob `id` as its viewers are told of it: dressed as it was stored (`look`), or as BDS
/// announced its kind, in `variant` and with its `properties`; either as it is `raised` now.
// TODO: a mob dressed as it was stored is not shown on fire to one who comes to see it burning.
pub(super) fn announcement(id: u64, kind: &str, body: &Body, (look, raised, properties, strides, strength): Dress<'_>, health: Option<&Health>, [yaw, pitch]: [f32; 2], (variant, size): (i32, Option<Gene>)) -> Vec<Bytes> {
    let left = health.map(Health::left);
    let saddle = raised.filter(|raised| raised.saddled).map_or(0, |_| crate::riding::saddle_flags(kind).0);
    let extra = MetadataFlags1(raised.map_or(0, |raised| raised.unstored_flags().0) | saddle);
    let dressed = look.map(|look| raised.map_or_else(|| look.clone(), |raised| raised.dress(look)));
    let told = properties.map(|properties| properties.shown(kind));
    let colour2 = raised.map_or(0, |raised| raised.kept.colour2);
    if let Some(shown) = dressed.and_then(|look| mobs::announcement_with(&Actor { position: body.at, health: left.or(look.health), ..look }, id, extra, told, (strides, strength))) {
        return super::looks::dressed(shown, kind, colour2);
    }
    let on_fire = health.is_some_and(Health::is_burning);
    let fire = if on_fire { MetadataFlags1::ONFIRE.0 } else { 0 };
    let kept = raised.map(|raised| &raised.kept);
    let options = AddEntityOptions {
        id,
        identifier: kind,
        feet: body.at,
        yaw,
        pitch,
        variant,
        // A new mob is announced before it is raised (birth.rs); a mooshroom's -1 shows then too.
        mark_variant: kept.map_or_else(|| Raised::newcomer(kind).kept.mark_variant, |kept| kept.mark_variant),
        color: kept.map_or(0, |kept| kept.colour),
        baby: raised.is_some_and(Raised::is_baby),
        flags: MetadataFlags1(fire | saddle | raised.map_or(0, |raised| raised.flags().0)),
        health: left,
        name: kept.map_or("", |kept| kept.name.as_str()),
    };
    let mut added = sapling_data::add_entity(&options);
    if let Some(strides) = strides {
        sapling_data::dress_strides(&mut added, strides, left);
    }
    if let Some(strength) = strength {
        crate::riding::dress_strength(&mut added, strength);
    }
    crate::riding::dress_inventory(&mut added, kind, raised.is_some_and(Raised::is_tamed), raised.is_some_and(Raised::is_baby), raised.is_some_and(|raised| raised.ungrouped));
    super::slimes::sizes::dress(&mut added, kind, size);
    super::properties::dress(&mut added, properties);
    if options.baby {
        dress_baby_look(&mut added, kind);
    }
    let mut shown = vec![sapling_net::encode(&added)];
    let held = HOLDS.iter().find(|(of, _)| *of == kind).map(|(_, item)| sapling_data::Stack { name: item, count: 1, damage: 0, block: None, tag: None });
    shown.extend(held.and_then(|stack| sapling_data::mob_hand(id, &stack, false)).map(|held| sapling_net::encode(&held)));
    super::looks::dressed(shown, kind, colour2)
}

/// A baby's scale and box in its announcement, for every kind: BDS's `AddEntity` carries them, where the
/// kind's recorded announcement may be a grown one's (a baby villager's 0.5, 0.98 by 1.96, bds-an1grow.txt,
/// bds-ar2look.txt).
fn dress_baby_look(added: &mut acacia_proto::packets::AddEntity, kind: &str) {
    use acacia_proto::types::{MetadataDictionaryItem as Item, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
    let Some(([width, height], scale)) = sapling_data::mob_look(kind, true) else { return };
    for (key, value) in [(Key::Scale, scale), (Key::BoundingboxWidth, width), (Key::BoundingboxHeight, height)] {
        let item = Item { key, r#type: Type::Float, legacy_type: 3, value: Value::Default(Plain::Float(value)) };
        match added.metadata.iter_mut().find(|old| old.key == key) {
            Some(old) => *old = item,
            None => added.metadata.push(item),
        }
    }
}

/// What tells a viewer of the mob now: dressed as stored and raised, its health and facing.
fn shown_now(world: &World, entity: Entity) -> Option<Vec<Bytes>> {
    let (id, kind, body) = (world.get::<NetId>(entity)?.0, &world.get::<entities::Kind>(entity)?.0, world.get::<Body>(entity)?);
    let raised = world.get::<Raised>(entity);
    let look = world.get::<Creature>(entity).and_then(|creature| creature.look.as_ref());
    let yaw = world.get::<Legs>(entity).map_or(0.0, |legs| legs.yaw);
    let pitch = world.get::<Told>(entity).map_or(0.0, |told| told.facing()[1]);
    let strides = world.get::<super::strides::Strode>(entity).map(|strode| &strode.0);
    let mut shown = announcement(id, kind, body, (look, raised, world.get::<Properties>(entity), strides, crate::riding::strength_of(world, entity)), world.get::<Health>(entity), [yaw, pitch], {
        let gene = world.get::<Gene>(entity).copied();
        (super::genes::variant(kind, gene), gene)
    });
    shown.extend(super::drowned::held(world, entity, id));
    shown.extend(super::piglins::held(world, entity, id));
    shown.extend(super::captains::worn(world, entity, id));
    shown.extend(super::worn::shown(world, entity, id));
    shown.extend(super::command_gear::shown(world, entity, id));
    let flags = world.get::<Told>(entity).and_then(Told::mob_flags);
    shown.extend(flags.and_then(|flags| super::husbandry::links_packet(world, entity, id, flags)));
    Some(super::villagers::dressed(world, entity, shown))
}

pub(crate) fn is_baby(world: &World, entity: Entity) -> bool {
    world.get::<Raised>(entity).is_some_and(Raised::is_baby)
}

/// A baby's pace over a grown one's of `kind`, as both were announced: BDS's baby zombies hunt at
/// their 0.35 (bds-q3baby.txt).
pub(super) fn pace_share(world: &World, entity: Entity, kind: &str) -> f32 {
    let paces = (sapling_data::mob_pace(kind, true), sapling_data::mob_pace(kind, false));
    match paces {
        (Some(baby), Some(grown)) if grown > 0.0 && is_baby(world, entity) => baby / grown,
        _ => 1.0,
    }
}

/// Players who come to see the mob later are told of it as it is now.
pub(super) fn reshow(world: &mut World, entity: Entity) {
    if let Some(shown) = shown_now(world, entity) {
        mobs::reannounce(world, entity, shown);
    }
    if let Some(effects) = super::afflictions::visible(world, entity) {
        mobs::reannounce_item(world, entity, effects);
    }
    if let Some(flags) = super::struck::shown_flags(world, entity) {
        mobs::reannounce_item(world, entity, flags);
    }
    if let Some(flags) = crate::riding::ghast_flags(world, entity) {
        mobs::reannounce_item(world, entity, flags);
    }
}

/// Where a mob's head is, as BDS places it; it faces +z (yaw 0).
pub(crate) fn head(kind: &str, body: &Body) -> [f32; 3] {
    let ([x, y, z], [width, height]) = (body.at, body.size);
    let forward = if ANIMALS.contains(&kind) { HEAD_FORWARD * width } else { 0.0 };
    [x, y + HEAD_UP * height, z + forward]
}

/// The sound `name` a mob makes as it appears, at its head, a baby's for a `baby`: `spawn`, or a
/// split slime's or bred baby's `born`.
pub(super) fn spawn_sound(name: &str, id: u64, kind: &str, body: &Body, baby: bool) -> Bytes {
    let mut sized = *body;
    // At the scaled box's head: a baby llama heard 0.795 up (1.87 × 0.5 × 0.85, bds-llamabreed.txt), a villager 0.833.
    sized.size = sapling_data::mob_look(kind, baby).filter(|_| baby).map_or(body.size, |(size, scale)| size.map(|side| side * scale));
    let [x, y, z] = head(kind, &sized);
    let position = Vec3f { x, y, z };
    let sound = LevelSoundEvent { sound_id: name.into(), position, extra_data: -1, entity_type: kind.into(), is_baby_mob: baby, is_global: false, entity_unique_id: id as i64, fire_at_position: None };
    sapling_net::encode(&sound)
}

/// How a mob of `kind` falls and slows: as most mobs, but for those that leap, fly and swim.
pub(super) fn physics_of(kind: &str) -> &'static Physics {
    match kind {
        "minecraft:breeze" => &Physics::BREEZE,
        "minecraft:ghast" | "minecraft:happy_ghast" => &Physics::FLYER,
        super::wither::KIND => &Physics::WITHER,
        vexes::KIND => &Physics::GHOST,
        "minecraft:turtle" => &Physics::TURTLE,
        "minecraft:bat" => &Physics::BAT,
        "minecraft:chicken" => &Physics::CHICKEN,
        "minecraft:allay" | super::bees::KIND => &Physics::HOVER,
        "minecraft:parrot" => &Physics::PARROT,
        "minecraft:squid" | "minecraft:glow_squid" => &Physics::SWIMMER,
        _ if super::swimming::is_fish(kind) => &Physics::FISH,
        _ if sapling_data::mob_traits(kind).swims.is_some() => &Physics::SWIMMER,
        _ => &Physics::MOB,
    }
}
