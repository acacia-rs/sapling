//! A thrown potion breaking by a player (docs/research/mob-attacks.md, "The witch"): a direct hit
//! gave the potion's effects in full, and harming threw the player as a hit does.
// TODO: one that breaks nearby gives the Java game's share, 1 less a quarter of the distance, of
// the time, and an instant effect in full; BDS's share off a hit was not measured.

use bevy_ecs::prelude::*;
use sapling_api::Effect;

use super::{Vitals, effects, is_mortal, struck};
use crate::components::NetId;

/// Effects shorter than this, shared out, are not given.
const SHORTEST: f32 = 20.0;

/// The potion of data value `data` broke `share` of its full strength on a player, thrown from `from`.
pub(crate) fn splashed(world: &mut World, player: Entity, data: i16, share: f32, from: [f32; 3]) {
    if !is_mortal(world, player) {
        return;
    }
    for gives in sapling_data::potion_effects(data) {
        let Some(effect) = Effect::ALL.into_iter().find(|effect| effect.name() == gives.effect) else { continue };
        let ticks = gives.ticks as f32 * share;
        if !effect.is_instant() && ticks < SHORTEST {
            continue;
        }
        if effect == Effect::InstantDamage
            && let Some(id) = world.get::<NetId>(player).map(|id| id.0)
        {
            struck::throw_from(world, player, id, from);
        }
        effects::give(world, player, effect, (ticks + 0.5) as u32, gives.amplifier, true);
    }
}

/// Whether the effect named `effect` is on the player.
pub(crate) fn is_under(world: &World, player: Entity, effect: &str) -> bool {
    Effect::ALL.into_iter().find(|known| known.name() == effect).is_some_and(|effect| effects::level(world, player, effect).is_some())
}

pub(crate) fn health_left(world: &World, player: Entity) -> f32 {
    world.get::<Vitals>(player).map_or(0.0, |vitals| vitals.health.left())
}
