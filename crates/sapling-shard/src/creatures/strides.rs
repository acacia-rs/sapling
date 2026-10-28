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

/// `compound` keeping `strides`.
pub(super) fn keep(compound: Vec<u8>, strides: Option<Strides>) -> Vec<u8> {
    match strides {
        Some(strides) => sapling_world::attributed_actor(&compound, &sapling_data::stride_attributes(&strides)).unwrap_or(compound),
        None => compound,
    }
}
