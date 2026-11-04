//! A mount's strides (`sapling_data::Strides`): rolled at its birth, kept in its compound's
//! `Attributes` as BDS keeps them, and read back from there.

use bevy_ecs::prelude::*;
use sapling_data::Strides;

use crate::blocks::Dice;

/// The strides a mob was born with.
#[derive(Component, Clone, Copy)]
pub(super) struct Strode(pub Strides);

/// The strides a newborn of `kind` rolls, if its kind has any.
pub(super) fn roll(world: &mut World, kind: &str) -> Option<Strides> {
    if !sapling_data::has_strides(kind) {
        return None;
    }
    let mut dice = world.get_resource_or_insert_with(Dice::default);
    let draws = [dice.fraction(), dice.fraction(), dice.fraction()];
    sapling_data::roll_strides(kind, draws)
}

/// The strides of a stored mob of `kind`: those its compound kept, or rolled now with the
/// compound to keep them where it kept none.
pub(super) fn stored(world: &mut World, kind: &str, compound: Vec<u8>) -> (Option<Strides>, Vec<u8>) {
    if !sapling_data::has_strides(kind) {
        return (None, compound);
    }
    if let Some(kept) = sapling_data::kept_strides(&sapling_world::actor_attributes(&compound)) {
        return (Some(kept), compound);
    }
    let rolled = roll(world, kind);
    let compound = keep(compound, rolled);
    (rolled, compound)
}

/// A foal of `kind` takes its `parents`' strides blended, where both are of its kind: its health,
/// its kept compound and its announcement follow (`sapling_data::blend_strides`).
pub(super) fn blend(world: &mut World, foal: Entity, parents: [Entity; 2], kind: &str) {
    let same = parents.iter().all(|parent| world.get::<crate::entities::Kind>(*parent).is_some_and(|of| of.0 == kind));
    let [Some(a), Some(b)] = parents.map(|parent| world.get::<Strode>(parent).map(|strode| strode.0)) else { return };
    if !same {
        return;
    }
    let draws = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        std::array::from_fn(|_| std::array::from_fn(|_| dice.fraction()))
    };
    let Some(blended) = sapling_data::blend_strides(kind, &a, &b, draws) else { return };
    world.entity_mut(foal).insert(Strode(blended));
    if let Some(mut health) = world.get_mut::<super::Health>(foal) {
        health.grow(blended.health);
        health.mend(blended.health);
    }
    if let Some(mut stored) = world.get_mut::<crate::entities::Stored>(foal) {
        stored.compound = keep(std::mem::take(&mut stored.compound), Some(blended));
    }
    super::shown::reshow(world, foal);
}

/// `compound` keeping `strides`.
pub(super) fn keep(compound: Vec<u8>, strides: Option<Strides>) -> Vec<u8> {
    match strides {
        Some(strides) => sapling_world::attributed_actor(&compound, &sapling_data::stride_attributes(&strides)).unwrap_or(compound),
        None => compound,
    }
}
