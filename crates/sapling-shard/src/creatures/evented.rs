//! A pack event fired on a living mob by `/event entity` (research/captures/bds-bb1event.txt): one its
//! definition names, exactly as named, fires; any other is refused.
// TODO: BDS answers the command before the event's SetEntityData (a tick's end); sapling tells it first.

use bevy_ecs::prelude::*;

use super::birth::{BABY_EVENTS, fire_herd_event};
use super::{husbandry, villagers};

/// Fires `event` on `entity` of `kind`, doing what sapling models of it: a baby event makes it a newborn,
/// a look or rider event as a herd's does, a `become_<profession>` retrains a villager. False where its
/// kind's definition has no such event.
// TODO: the events' other component groups and properties (a sheep's `temperate_color` sets its Color).
pub(super) fn fire(world: &mut World, entity: Entity, kind: &str, event: &str) -> bool {
    if !sapling_data::has_event(kind, event) {
        return false;
    }
    // BDS told a baby sheep nothing of `spawn_baby` or `entity_born` fired again.
    if BABY_EVENTS.contains(&event.trim_start_matches("minecraft:")) && !super::shown::is_baby(world, entity) {
        husbandry::be_born(world, entity, kind);
    }
    fire_herd_event(world, entity, kind, event);
    villagers::retrain(world, entity, event);
    true
}
