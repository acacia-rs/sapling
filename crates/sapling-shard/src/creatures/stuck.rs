//! Shots that stuck where they met a block (flight.rs): those a player may pick up are kept with
//! their column and come back from the world folder stuck.

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
    world.entity_mut(arrow).insert((Arrow { archer: String::new(), by: arrow, gives: None, shot, born: now, stuck: Some(now), summoned: false }, Loosed { player: None, pickup, item, tipped, ..Loosed::new(arrow, pickup, None) }));
    entities::keep(world, arrow, layer, unique_id, compound);
    true
}

/// The stuck shots a player may pick up: each, its world, where it is, its id and what it carries.
pub(crate) fn lying(world: &mut World) -> Vec<(Entity, Entity, [f32; 3], u64, Loosed)> {
    let mut shots = world.query::<(Entity, &InLayer, &Body, &NetId, &Loosed, &Arrow)>();
    let lies = |loosed: &Loosed, arrow: &Arrow| loosed.pickup != Pickup::Never && !loosed.returning && arrow.stuck.is_some();
    shots.iter(world).filter(|(.., loosed, arrow)| lies(loosed, arrow)).map(|(arrow, layer, body, id, loosed, _)| (arrow, layer.0, body.at, id.0, loosed.clone())).collect()
}

/// The shot frees itself from where it was stuck: it flies again (a trident coming back).
pub(crate) fn unstick(world: &mut World, arrow: Entity) {
    if let Some(mut arrow) = world.get_mut::<Arrow>(arrow) {
        arrow.stuck = None;
    }
}
