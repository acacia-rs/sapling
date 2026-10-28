//! Mobs that hatch from a block (blocks/eggs.rs): each comes as a newborn baby of its kind, through
//! its `entity_born`; a turtle keeps the egg's cell as its home (`minecraft:home`'s `HomePos`).

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, SpawnReason};

use crate::components::{InLayer, Layer};
use crate::entities;

/// A baby of `kind` hatches with its feet at `feet`.
pub(crate) fn hatch(world: &mut World, layer: Entity, kind: &str, feet: [f32; 3]) -> Option<Entity> {
    let baby = super::spawn_entity(world, layer, kind, feet, SpawnReason::Hatched)?;
    super::husbandry::be_born(world, baby, kind);
    Some(baby)
}

/// The mob calls the cell `home` of its world home, and its stored compound keeps it.
pub(crate) fn make_home(world: &mut World, mob: Entity, home: BlockPos) {
    let Some(dimension) = world.get::<InLayer>(mob).and_then(|layer| world.get::<Layer>(layer.0)).map(|layer| layer.world.dimension()) else { return };
    entities::restore(world, mob, |compound, _| sapling_world::homed_actor(compound, home.map(|c| c as f32), dimension).ok());
}
