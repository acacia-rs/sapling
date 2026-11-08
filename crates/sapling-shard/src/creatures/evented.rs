//! A pack event fired by `/event entity` (research/captures/bds-bb1event.txt): one the target's
//! definition names, exactly as named, fires; any other is refused. What it changes is told after the
//! command's answer (entities::told_after_answer).

use bevy_ecs::prelude::*;

use super::birth::{BABY_EVENTS, fire_herd_event};
use super::{husbandry, raids, villagers};

const PLAYER: &str = "minecraft:player";

/// Fires `event` on `entity` of `kind`, doing what sapling models of it: a baby event makes it a newborn,
/// a look or rider event as a herd's does, a `become_<profession>` retrains a villager, a raised
/// mob's growing, wool and colour events (husbandry/fired.rs) and a creeper's charge. False where its kind's definition has no
/// such event.
// TODO: the events' other component groups and properties.
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
    husbandry::fired(world, entity, kind, event);
    super::struck::evented(world, entity, kind, event);
    true
}

/// [`fire`] on a player: `minecraft:gain_raid_omen` gives it the raid omen as a bad omen in a
/// village does; the player's other events change nothing a drill showed (bds-bc1event.txt).
pub(super) fn fire_on_player(world: &mut World, player: Entity, event: &str) -> bool {
    if !sapling_data::has_event(PLAYER, event) {
        return false;
    }
    if event == "minecraft:gain_raid_omen" {
        raids::gain_raid_omen(world, player);
    }
    true
}
