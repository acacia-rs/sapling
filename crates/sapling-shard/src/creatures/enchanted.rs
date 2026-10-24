//! What a weapon's enchantments do to a mob it hit beyond the harm (attacks.rs), and a player's
//! Thorns on a mob that hit it (`crate::enchants::thorns`). The rules: `sapling_data::enchant`.

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_api::Effect;
use sapling_data::enchant::{self, Enchant};

use super::health::{Cause, Health};
use super::{afflictions, harm};
use crate::blocks::Dice;
use crate::components::InLayer;

/// A mob of `kind` was hit by `weapon`: Fire Aspect sets it alight, Bane of Arthropods slows an
/// arthropod.
pub(super) fn struck(world: &mut World, mob: Entity, kind: &str, weapon: Option<&Value>) {
    let burns = enchant::aspect_burn(enchant::level(weapon, Enchant::FireAspect));
    if let Some(mut health) = world.get_mut::<Health>(mob).filter(|_| burns > 0) {
        health.ignite(burns);
    }
    let bane = enchant::level(weapon, Enchant::BaneOfArthropods);
    if bane > 0 && sapling_data::is_arthropod(kind) {
        let ticks = {
            let mut dice = world.get_resource_or_insert_with(Dice::default);
            enchant::bane_slowness(bane, &mut || dice.fraction())
        };
        afflictions::give(world, mob, Effect::Slowness, ticks, enchant::BANE_SLOWNESS);
    }
}

/// The Thorns of a player `mob` hit hurt it for `amount`.
pub(crate) fn thorned(world: &mut World, mob: Entity, amount: f32) {
    if world.get::<Health>(mob).is_some()
        && let Some(layer) = world.get::<InLayer>(mob).map(|layer| layer.0)
    {
        harm::wound(world, mob, layer, amount, Cause::Thorns);
    }
}
