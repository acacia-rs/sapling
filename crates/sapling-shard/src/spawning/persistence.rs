//! Whether a mob stays, through a world folder: BDS saves a mob's `PersistentComponent` as
//! `Persistent` and reads it back (`DespawnComponent::_canDespawn` 0x141ba30e0 asks it), and
//! keeps `NaturalSpawn` beside it. A mob that appeared by itself ([`Natural`]) is saved with
//! `Persistent` 0 and `NaturalSpawn` 1 and comes back despawning; any other with `Persistent` 1
//! (summoned: research/captures/bds-keepmobs.txt). docs/research/mob-spawning.md "Despawning".

use bevy_ecs::prelude::*;

use super::Natural;
use crate::creatures::Raised;

/// Before the mobs are written back: each one's kept `Persistent` and `NaturalSpawn` as it is now.
pub(crate) fn mark(world: &mut World) {
    let mut mobs = world.query::<(&mut Raised, Has<Natural>)>();
    for (mut raised, natural) in mobs.iter_mut(world) {
        raised.kept.persistent = !natural;
        raised.kept.natural |= natural;
    }
}

/// A mob made from its stored compound despawns again if it was saved as a natural one that
/// nothing kept. One saved with no `NaturalSpawn` (sapling before 2026-11-05) stays.
pub(crate) fn adopt(world: &mut World, entity: Entity) {
    if world.get::<Raised>(entity).is_some_and(|raised| raised.kept.natural && !raised.kept.persistent) {
        world.entity_mut(entity).insert(Natural);
    }
}
