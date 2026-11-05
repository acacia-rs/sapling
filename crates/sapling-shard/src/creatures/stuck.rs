//! Shots that stuck where they met a block (flight.rs): those a player may pick up are kept with
//! their column and come back from the world folder stuck.

use acacia_physics::WorldView;
use acacia_proto::packets::{EntityEvent, EntityEventEventId};
use acacia_proto::types::MetadataFlags1;
use bevy_ecs::prelude::*;
use sapling_data::AddEntityOptions;
use sapling_world::{ItemStack, ShotRecord};

use super::arrows::Shot;
use super::flight::{Arrow, Loosed, Pickup};
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Tick};
use crate::entities;
use crate::sync::Told;

const SHAKES: i32 = 7;

/// Has the world folder keep a stuck shot a player may pick up, with its column.
pub(super) fn keep(world: &mut World, arrow: Entity, layer: Entity, body: &Body) -> Option<()> {
    let loosed = world.get::<Loosed>(arrow).filter(|loosed| loosed.pickup != Pickup::Never)?.clone();
    let kind = world.get::<Arrow>(arrow)?.shot.kind;
    let unique_id = entities::new_unique_id(world, arrow)?;
    let rotation = world.get::<Told>(arrow)?.facing();
    let trident = loosed.item.filter(|item| item.name != super::arrows::ARROW.kind);
    let record = ShotRecord { player: loosed.pickup == Pickup::Anyone, creative: loosed.pickup == Pickup::Creative, aux: loosed.tipped.map_or(0, |data| (data + 1) as i8), trident };
    let compound = sapling_world::new_shot_actor(unique_id, kind, body.at, rotation, &record, world.get::<Layer>(layer)?.world.registry());
    entities::keep(world, arrow, layer, unique_id, compound);
    Some(())
}

/// A shot a world folder kept lies stuck in `layer` again; false for one no player may pick up.
pub(crate) fn from_storage(world: &mut World, layer: Entity, kind: &str, at: [f32; 3], [yaw, pitch]: [f32; 2], unique_id: i64, compound: Vec<u8>, shot: Shot) -> bool {
    let Some(record) = sapling_world::shot_record(&compound) else { return false };
    let pickup = match (record.player, record.creative) {
        (true, _) => Pickup::Anyone,
        (false, true) => Pickup::Creative,
        _ => return false,
    };
    let tipped = (record.aux > 0).then(|| i16::from(record.aux) - 1);
    let item = record.trident.or_else(|| Some(ItemStack { name: kind.to_owned(), count: 1, damage: tipped.map_or(0, |data| data + 1), block: None, tag: None }));
    let Some(arrow) = entities::spawn(world, layer, kind, Body::new(at, [0.0; 3], shot.size), at, |id| {
        let options = AddEntityOptions { id, identifier: kind, feet: at, yaw, pitch, variant: 0, mark_variant: 0, color: 0, baby: false, flags: MetadataFlags1(0), health: None, name: "" };
        Some(vec![sapling_net::encode(&sapling_data::add_entity(&options))])
    }) else { return false };
    let now = world.resource::<Tick>().0;
    world.entity_mut(arrow).insert((Arrow { archer: String::new(), by: arrow, gives: None, shot, born: now, stuck: Some(now), summoned: false, bounced_off: None }, Loosed { player: None, pickup, item, tipped, ..Loosed::new(arrow, pickup, None) }));
    entities::keep(world, arrow, layer, unique_id, compound);
    true
}

/// The stuck shots a player may pick up: each, its world, where it is, its id and what it carries.
pub(crate) fn lying(world: &mut World) -> Vec<(Entity, Entity, [f32; 3], u64, Loosed)> {
    let mut shots = world.query::<(Entity, &InLayer, &Body, &NetId, &Loosed, &Arrow)>();
    let lies = |loosed: &Loosed, arrow: &Arrow| loosed.pickup != Pickup::Never && !loosed.returning && arrow.stuck.is_some();
    shots.iter(world).filter(|(.., loosed, arrow)| lies(loosed, arrow)).map(|(arrow, layer, body, id, loosed, _)| (arrow, layer.0, body.at, id.0, loosed.clone())).collect()
}

/// Whether the stuck shot `arrow` still shakes from its hit and is not to be had: BDS's creative
/// watcher took one 8 ticks after it lay still (research/captures/bds-f1pickup.txt).
pub(crate) fn shaking(world: &World, arrow: Entity) -> bool {
    const SHAKING: u64 = 8;
    let now = world.resource::<Tick>().0;
    world.get::<Arrow>(arrow).and_then(|arrow| arrow.stuck).is_some_and(|stuck| now < stuck + SHAKING)
}

/// Whether nothing holds the stuck shot `arrow` any more: no block's collision box touches it, as
/// when its block is set to air. BDS's then lies a tick more and falls from rest (bds-g10flame.txt).
pub(super) fn loosened(world: &World, arrow: Entity, layer: Entity) -> bool {
    let (Some(body), Some(level)) = (world.get::<Body>(arrow), world.get::<Layer>(layer)) else { return false };
    let near = acacia_physics::Aabb { min: body.at, max: body.at }.grow(1e-3);
    let mut boxes = Vec::new();
    crate::terrain::physics_view(&level.world, |view| view.collisions(&near, &mut boxes));    boxes.is_empty()
}

/// Whether a shot at `at` moving by `motion` bounces off `struck`: primed TNT it flies into, which
/// BDS's arrow left a tenth as fast the other way (bds-g10flame.txt).
pub(super) fn bounces_off(world: &World, struck: Entity, at: [f32; 3], motion: [f32; 3]) -> bool {
    let Some(tnt) = world.get::<Body>(struck).filter(|_| world.get::<crate::tnt::Primed>(struck).is_some()) else { return false };
    let towards = (tnt.at[0] - at[0]) * motion[0] + (tnt.at[2] - at[2]) * motion[2];
    towards > 0.0
}

/// What BDS tells of a shot's hit on a block, the tick after (bds-g10flame.txt): its sound, then an
/// arrow's shake where it is `held`.
pub(super) fn tell_hit(world: &mut World, layer: Entity, id: u64, at: [f32; 3], shot: Shot, held: bool) {
    let cell = at.map(|c| c.floor() as i32);
    crate::late_sounds::play_later(world, 1, layer, cell, super::flight::sound(shot.heard[1], at, ""));
    if held && shot.kind == super::arrows::ARROW.kind {
        let shake = EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::ArrowShake, data: SHAKES, fire_at_position: None };
        crate::late_sounds::play_later(world, 1, layer, cell, sapling_net::encode(&shake));
    }
}

/// The shot frees itself from where it was stuck: it flies again (a trident coming back).
pub(crate) fn unstick(world: &mut World, arrow: Entity) {
    if let Some(mut arrow) = world.get_mut::<Arrow>(arrow) {
        arrow.stuck = None;
    }
}
