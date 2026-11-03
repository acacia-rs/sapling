//! A mob's effects in the world folder, kept as BDS keeps them (`ActiveEffects`,
//! research/captures/bds-keepmobs.txt).

use bevy_ecs::prelude::*;
use sapling_api::Effect;

use super::{Active, Afflicted};
use crate::entities;

/// Puts the effects a world folder kept with the mob (its compound `compound`) back on it.
pub(in crate::creatures) fn adopt(world: &mut World, mob: Entity, compound: &[u8]) {
    let active = |[id, amplifier, left, shown]: [i32; 4]| {
        let effect = Effect::ALL.into_iter().find(|known| known.id() == id)?;
        Some(Active { effect, amplifier: u8::try_from(amplifier).ok()?, left: u32::try_from(left).ok().filter(|left| *left > 0)?, particles: shown != 0 })
    };
    let kept: Vec<Active> = sapling_world::actor_effects(compound).into_iter().filter_map(active).collect();
    if !kept.is_empty() {
        world.entity_mut(mob).insert(Afflicted(kept));
    }
}

/// Writes each kept mob's effects into what the world folder keeps of it.
pub(in crate::creatures) fn keep(world: &mut World) {
    let afflicted: Vec<(Entity, Vec<[i32; 4]>)> = world
        .query_filtered::<(Entity, &Afflicted), With<entities::Stored>>()
        .iter(world)
        .map(|(mob, afflicted)| (mob, afflicted.0.iter().map(|active| [active.effect.id(), i32::from(active.amplifier), active.left as i32, i32::from(active.particles)]).collect()))
        .collect();
    for (mob, effects) in afflicted {
        entities::restore(world, mob, |compound, _| sapling_world::effected_actor(compound, &effects).ok());
    }
}
