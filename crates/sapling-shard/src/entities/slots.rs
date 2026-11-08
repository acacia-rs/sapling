//! BDS's actor storage order. Every actor (player, mob, item, orb) holds a slot of the registry's
//! `ActorOwnerComponent` storage, an EnTT storage with in-place delete: a new actor takes the slot freed
//! last, or one past the end when none is free; a gone one frees its slot. The tick's actor-data sync
//! (`_tickActorDataSyncSystem`, ELF 0x6d5d150) walks that storage from its last slot down, so every
//! tick-end SetEntityData goes out highest slot first (docs/research/villagers.md "Actor order").
// TODO: BDS frees a tick's dead per chunk in its shuffled chunk order; sapling frees in despawn order.

use std::collections::HashMap;

use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::prelude::*;
use bevy_ecs::world::DeferredWorld;

use crate::components::NetId;

#[derive(Resource, Default)]
pub(crate) struct Slots {
    of: HashMap<Entity, usize>,
    end: usize,
    /// Freed slots, the last freed on top.
    free: Vec<usize>,
}

impl Slots {
    fn take(&mut self, entity: Entity) {
        let slot = self.free.pop().unwrap_or_else(|| {
            self.end += 1;
            self.end - 1
        });
        self.of.insert(entity, slot);
    }

    fn release(&mut self, entity: Entity) {
        if let Some(slot) = self.of.remove(&entity) {
            self.free.push(slot);
        }
    }

    pub(crate) fn of(&self, entity: Entity) -> Option<usize> {
        self.of.get(&entity).copied()
    }
}

/// `NetId`'s add hook: the new actor takes its slot.
pub(crate) fn take(mut world: DeferredWorld, context: HookContext) {
    if let Some(mut slots) = world.get_resource_mut::<Slots>() {
        slots.take(context.entity);
    }
}

/// `NetId`'s remove hook: the gone actor frees its slot.
pub(crate) fn release(mut world: DeferredWorld, context: HookContext) {
    if let Some(mut slots) = world.get_resource_mut::<Slots>() {
        slots.release(context.entity);
    }
}

/// The storage slot of `entity`.
pub(crate) fn slot(world: &World, entity: Entity) -> Option<usize> {
    world.get_resource::<Slots>()?.of(entity)
}

/// The storage slot of the actor whose runtime id is `id`.
pub(crate) fn of_id(world: &mut World, id: u64) -> Option<usize> {
    let entity = world.query::<(Entity, &NetId)>().iter(world).find(|(_, net)| net.0 == id).map(|(entity, _)| entity)?;
    world.get_resource::<Slots>()?.of(entity)
}

/// `told` (what is about an actor, by its runtime id) in BDS's sync order: highest slot first, the
/// packets of one actor in the order they came; those about no known actor last.
pub(crate) fn in_sync_order<T>(world: &mut World, told: Vec<(Option<u64>, T)>) -> Vec<T> {
    let mut slot_of: HashMap<u64, Option<usize>> = HashMap::new();
    let mut keyed: Vec<(Option<usize>, T)> = told.into_iter().map(|(id, item)| (id.and_then(|id| *slot_of.entry(id).or_insert_with(|| of_id(world, id))), item)).collect();
    keyed.sort_by_key(|(slot, _)| std::cmp::Reverse(slot.map_or(-1, |slot| slot as i64)));
    keyed.into_iter().map(|(_, item)| item).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// be3slots' last kill: slots 3, 14 and 7 freed in that order; the next three actors took 7, 14, 3.
    #[test]
    fn a_new_actor_takes_the_slot_freed_last() {
        let mut world = World::new();
        let entities: Vec<Entity> = (0..16).map(|_| world.spawn_empty().id()).collect();
        let mut slots = Slots::default();
        entities.iter().for_each(|entity| slots.take(*entity));
        for gone in [3, 14, 7] {
            slots.release(entities[gone]);
        }
        let new: Vec<Entity> = (0..4).map(|_| world.spawn_empty().id()).collect();
        new.iter().for_each(|entity| slots.take(*entity));
        assert_eq!(new.iter().map(|entity| slots.of(*entity)).collect::<Vec<_>>(), [Some(7), Some(14), Some(3), Some(16)]);
    }
}
