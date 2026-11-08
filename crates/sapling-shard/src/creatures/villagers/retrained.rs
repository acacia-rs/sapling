//! A villager given a profession by its pack's `minecraft:become_<group>` fired on it (`/event entity`):
//! it takes up the profession with its table's offers; BDS tells only the new Variant
//! (research/captures/bds-bb1event.txt).

use acacia_proto::types::MetadataDictionaryItemKey as Key;
use bevy_ecs::prelude::*;

use super::{Trader, deals, int, jobs, tell};
use crate::blocks::Dice;
use crate::components::InLayer;

/// Fires `event` on the trader `entity`; nothing for an event that names no profession.
pub(in crate::creatures) fn fired(world: &mut World, entity: Entity, event: &str) {
    let Some(variant) = jobs::of_event(event) else { return };
    let Some(mut trader) = world.get::<Trader>(entity).cloned() else { return };
    let mut dice = world.remove_resource::<Dice>().unwrap_or_default();
    jobs::take_up(&mut trader, variant, &mut dice);
    world.insert_resource(dice);
    world.entity_mut(entity).insert(trader);
    if let Some(layer) = world.get::<InLayer>(entity).map(|layer| layer.0) {
        deals::chart(world, entity, layer);
    }
    tell(world, entity, vec![int(Key::Variant, variant)]);
}
