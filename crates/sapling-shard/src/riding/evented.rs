//! A mob saddled by its spawn event (research/captures/bds-saddleevents.txt): the horse family's and
//! a camel's `<kind>_saddled` show the saddle and its controls and are heard as `saddle`, yet hold
//! none, so none drops; and as the summon runs that event alone, never the one adding the
//! `<kind>_adult` group their loot table sits in, they leave no loot either. A pig's or strider's
//! `on_saddled` is unheard, and its death leaves the saddle with its loot as a hand-saddled one's
//! does (their `<kind>_saddled` loot table).

use bevy_ecs::prelude::*;
use sapling_world::ItemStack;

use crate::body::Body;
use crate::components::{InLayer, NetId};
use crate::creatures::Raised;

/// Each kind's saddling event.
const EVENTS: [(&str, &str); 7] = [
    ("minecraft:horse", "horse_saddled"),
    ("minecraft:zombie_horse", "horse_saddled"),
    ("minecraft:donkey", "donkey_saddled"),
    ("minecraft:mule", "mule_saddled"),
    ("minecraft:camel", "camel_saddled"),
    ("minecraft:pig", "on_saddled"),
    ("minecraft:strider", "on_saddled"),
];
/// Kinds whose saddle is loot: their event holds it too, and it drops only with `doMobLoot`.
const LOOTED: [&str; 2] = ["minecraft:pig", "minecraft:strider"];

/// A mount that shows a saddle its spawn event gave it, holds none and has no loot.
#[derive(Component)]
pub(crate) struct Bare;

/// Saddles a mob of `kind` summoned with `event` if that is its kind's saddling event; false otherwise.
pub(crate) fn saddle_by_event(world: &mut World, mob: Entity, kind: &str, event: &str) -> bool {
    if !EVENTS.contains(&(kind, event.trim_start_matches("minecraft:"))) {
        return false;
    }
    let (Some(id), Some(body), Some(layer)) = (world.get::<NetId>(mob).map(|id| id.0), world.get::<Body>(mob).copied(), world.get::<InLayer>(mob).map(|layer| layer.0)) else { return false };
    if let Some(mut raised) = world.get_mut::<Raised>(mob) {
        raised.saddled = true;
    }
    let looted = LOOTED.contains(&kind);
    if !looted {
        world.entity_mut(mob).insert(Bare);
    }
    let word = crate::entities::mob_flags(world, mob, kind, super::saddle_flags(kind), true);
    let mut told = vec![super::packets::flags(id, word)];
    told.extend((!looted).then(|| super::packets::sound("saddle", id, kind, body.at)));
    crate::entities::tell_viewers(world, mob, layer, body.at.map(|c| c.floor() as i32), told);
    super::retell(world, mob);
    true
}

/// The saddle a dying pig or strider leaves with its loot, if it wears one.
pub(crate) fn looted_saddle(world: &World, mob: Entity, kind: &str) -> Option<ItemStack> {
    (LOOTED.contains(&kind) && super::mounts::saddled(world, mob)).then(|| ItemStack { name: "minecraft:saddle".into(), count: 1, damage: 0, block: None, tag: None })
}

/// Whether the saddle `mob` shows is its spawn event's, held by nothing; such a mob has no loot.
pub(crate) fn is_bare(world: &World, mob: Entity) -> bool {
    world.get::<Bare>(mob).is_some()
}
