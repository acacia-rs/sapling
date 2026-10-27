//! Wither skulls, as BDS's pack has them (`wither_skull`, `wither_skull_dangerous`): straight, at
//! 1.2 a tick, a blue one at 0.6; one that meets a player or a block blows up there with power 1,
//! and gives a player it met Wither II for 10 seconds on normal, 40 on hard.
// TODO: a skull meets no mob; a blue one breaks no stronger block than a black one does
// (`max_resistance` 4), and is not struck back.

use acacia_proto::types::{MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1, Vec3f};
use bevy_ecs::prelude::*;
use sapling_api::{Difficulty, Effect};
use sapling_data::AddEntityOptions;

use super::super::blast::{self, Wreck};
use super::Blasting;
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Position, Settings, Tick, in_world};
use crate::sync::Told;
use crate::{entities, mobs, vitals};

const BLACK: &str = "minecraft:wither_skull";
const BLUE: &str = "minecraft:wither_skull_dangerous";
const SIZE: [f32; 2] = [0.3125, 0.3125];
const PLAYER: [f32; 2] = [0.6, 1.8];
const SPEED: f32 = 1.2;
const BLUE_SPEED: f32 = 0.6;
const POWER: f32 = 1.0;
/// Wither II, for this many ticks on normal and on hard.
const WITHERS: [u32; 2] = [200, 800];
const LEVEL: u8 = 1;
/// One that met nothing goes after this many ticks, so that misses do not pile up.
const FLIES: u64 = 200;
const GRIEFING: &str = "mobGriefing";

#[derive(Component)]
struct Skull {
    born: u64,
    wither: Entity,
}

/// The wither shoots a skull from `from` at `to`, a blue one if `blue`.
pub(super) fn fire(world: &mut World, layer: Entity, wither: Entity, from: [f32; 3], to: [f32; 3], blue: bool) {
    let aim: [f32; 3] = std::array::from_fn(|axis| to[axis] - from[axis]);
    let long = aim.iter().map(|c| c * c).sum::<f32>().sqrt().max(1e-4);
    let motion = aim.map(|c| c / long * if blue { BLUE_SPEED } else { SPEED });
    let owner = world.get::<NetId>(wither).map_or(-1, |id| id.0 as i64);
    let kind = if blue { BLUE } else { BLACK };
    let [yaw, pitch] = crate::creatures::facing(motion);
    let announce = |id: u64| {
        let options = AddEntityOptions { id, identifier: kind, feet: from, yaw, pitch, variant: 0, mark_variant: 0, color: 0, baby: false, flags: MetadataFlags1(0), health: None, name: "" };
        let mut shown = sapling_data::add_entity(&options);
        shown.velocity = Vec3f { x: motion[0], y: motion[1], z: motion[2] };
        shown.metadata.retain(|item| item.key != Key::OwnerEid);
        shown.metadata.push(MetadataDictionaryItem { key: Key::OwnerEid, r#type: Type::Long, legacy_type: 7, value: Value::Default(Plain::Long(owner)) });
        Some(vec![sapling_net::encode(&shown)])
    };
    let Some(skull) = entities::spawn(world, layer, kind, Body::new(from, motion, SIZE), from, announce) else { return };
    let born = world.resource::<Tick>().0;
    world.entity_mut(skull).insert(Skull { born, wither });
    entities::after_announcing(world, layer, from.map(|c| c.floor() as i32), vec![super::heads::sound("shoot", from)]);
}

/// The player of `layer` whose box the skull's, its bottom middle at `at`, reaches into.
fn met(world: &World, layer: Entity, at: [f32; 3]) -> Option<Entity> {
    let across = (PLAYER[0] + SIZE[0]) / 2.0;
    in_world(world).into_iter().find(|player| {
        let Some(feet) = world.get::<Position>(*player).map(|position| position.feet) else { return false };
        world.get::<InLayer>(*player).map(|layer| layer.0) == Some(layer) && (at[0] - feet[0]).abs() < across && (at[2] - feet[2]).abs() < across && at[1] + SIZE[1] > feet[1] && at[1] < feet[1] + PLAYER[1]
    })
}

/// The skull blows up at `at`, sparing its wither; a player it met withers.
fn burst(world: &mut World, skull: Entity, layer: Entity, at: [f32; 3], struck: Option<Entity>) {
    let wither = world.get::<Skull>(skull).map(|kept| kept.wither).filter(|wither| world.get_entity(*wither).is_ok());
    let withers = match world.resource::<Settings>().0.difficulty {
        Difficulty::Normal => Some(WITHERS[0]),
        Difficulty::Hard => Some(WITHERS[1]),
        _ => None,
    };
    if let (Some(player), Some(ticks)) = (struck, withers) {
        vitals::give_effect(world, player, Effect::Wither, ticks, LEVEL, true);
    }
    let griefs = world.get::<Layer>(layer).is_some_and(|layer| layer.world.level().rules.on(GRIEFING));
    if let Some(wither) = wither {
        world.entity_mut(wither).insert(Blasting);
    }
    blast::explode(world, Some(skull), layer, at, POWER, if griefs { Wreck::Decays } else { Wreck::Nothing });
    if let Some(wither) = wither {
        world.entity_mut(wither).remove::<Blasting>();
    }
}

/// A tick of one skull.
fn fly(world: &mut World, skull: Entity, layer: Entity, id: u64, now: u64) -> Option<()> {
    let mut body = *world.get::<Body>(skull)?;
    if now >= world.get::<Skull>(skull)?.born + FLIES {
        mobs::remove(world, skull);
        return None;
    }
    // Looked at a quarter of a step at a time: a whole one is longer than a player is across.
    let along = |part: f32| -> [f32; 3] { std::array::from_fn(|axis| body.at[axis] + body.motion[axis] * part) };
    if let Some((player, at)) = [0.25, 0.5, 0.75, 1.0].into_iter().find_map(|part| met(world, layer, along(part)).map(|player| (player, along(part)))) {
        burst(world, skull, layer, at, Some(player));
        return None;
    }
    if !Body::new(along(1.0), [0.0; 3], SIZE).is_free(&world.get::<Layer>(layer)?.world) {
        burst(world, skull, layer, body.at, None);
        return None;
    }
    body.at = along(1.0);
    *world.get_mut::<Body>(skull)? = body;
    let packets = world.get_mut::<Told>(skull)?.update(id, body.at, body.motion, false);
    entities::tell_viewers(world, skull, layer, body.at.map(|c| c.floor() as i32), packets);
    Some(())
}

/// Moves every skull a tick.
pub(super) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let skulls: Vec<(Entity, Entity, u64)> = world.query_filtered::<(Entity, &InLayer, &NetId), With<Skull>>().iter(world).map(|(skull, layer, id)| (skull, layer.0, id.0)).collect();
    for (skull, layer, id) in skulls {
        fly(world, skull, layer, id, now);
    }
}
