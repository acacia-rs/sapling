//! Babies: one grows up its kind's `minecraft:ageable` duration after it was born, sooner for each
//! of its foods it is fed (that food's share of the duration), and then has a grown one's box.

use acacia_proto::packets::EntityEventEventId;
use bevy_ecs::prelude::*;

use super::{Raised, regroup, shown};
use crate::body::Body;

/// `minecraft:ageable`'s duration where a kind names none: 1200 seconds.
pub(super) const GROWS: u32 = 24_000;

fn grows(kind: &str) -> u32 {
    sapling_data::husbandry(kind).and_then(|raised| raised.grows).unwrap_or(GROWS)
}

/// The mob's box as its age makes it: a baby's is the grown box at the baby's scale.
pub(super) fn fit_body(world: &mut World, entity: Entity, kind: &str) {
    let baby = world.get::<Raised>(entity).is_some_and(Raised::is_baby);
    let (Some(grown), Some((_, scale))) = (sapling_data::mob_size(kind), sapling_data::mob_look(kind, baby)) else { return };
    if let Some(mut body) = world.get_mut::<Body>(entity) {
        body.size = grown.map(|side| side * scale);
    }
}

/// Makes the mob a newborn baby of its kind, with the groups BDS's `entity_born` gives it.
pub(in crate::creatures) fn be_born(world: &mut World, entity: Entity, kind: &str) {
    if let Some(mut raised) = world.get_mut::<Raised>(entity) {
        raised.kept.baby = Some(grows(kind) as i32);
    }
    fit_body(world, entity, kind);
    shown::age(world, entity);
    if let Some(swap) = sapling_data::husbandry(kind).map(|raised| &raised.on_born) {
        regroup(world, entity, swap);
    }
    super::super::reshow(world, entity);
}

/// A tick of a baby's growing; one of a kind that does not age (a baby zombie's) stays one.
pub(super) fn tick(world: &mut World, entity: Entity, kind: &str) {
    if sapling_data::husbandry(kind).is_none_or(|raised| raised.grows.is_none()) {
        return;
    }
    let Some(mut raised) = world.get_mut::<Raised>(entity) else { return };
    let Some(left) = raised.kept.baby.as_mut() else { return };
    *left -= 1;
    if *left <= 0 {
        grow_up(world, entity, kind);
    }
}

fn grow_up(world: &mut World, entity: Entity, kind: &str) {
    if let Some(mut raised) = world.get_mut::<Raised>(entity) {
        raised.kept.baby = None;
    }
    fit_body(world, entity, kind);
    shown::age(world, entity);
    if let Some(swap) = sapling_data::husbandry(kind).map(|raised| &raised.on_grow) {
        regroup(world, entity, swap);
    }
    super::super::reshow(world, entity);
}

/// A baby is fed `item`: if it is one of its foods it grows that much sooner, and its viewers see
/// it eat. False for a grown one or an item it does not eat.
pub(super) fn feed(world: &mut World, entity: Entity, kind: &str, item: &str) -> bool {
    let Some(share) = sapling_data::husbandry(kind).and_then(|raised| raised.grow.iter().find(|(food, _)| *food == item).map(|(_, share)| *share)) else { return false };
    let Some(mut raised) = world.get_mut::<Raised>(entity) else { return false };
    let Some(left) = raised.kept.baby.as_mut() else { return false };
    *left -= (share * grows(kind) as f32) as i32;
    shown::event(world, entity, EntityEventEventId::BabyAnimalFeed, 0);
    true
}
