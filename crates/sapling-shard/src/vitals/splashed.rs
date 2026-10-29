//! A thrown potion breaking by a player (docs/research/mob-kinds.md, "The witch"): a direct hit
//! gave the potion's effects in full, and harming threw the player as a hit does.
// TODO: an instant effect from one that breaks nearby is given in full; BDS's share was not measured.

use bevy_ecs::prelude::*;
use sapling_api::Effect;

use super::{Vitals, effects, is_dead, is_mortal, struck};
use crate::components::{Mode, NetId};

/// A creative player is given a potion's effects too (bds-clouds.txt: the watcher's poison), only
/// not hurt; a spectator nothing.
fn is_reached(world: &World, player: Entity) -> bool {
    !is_dead(world, player) && world.get::<Mode>(player).is_some_and(|mode| mode.0 != sapling_api::GameMode::Spectator)
}

/// Effects shorter than this, shared out, are not given.
const SHORTEST: f32 = 20.0;

/// The potion of data value `data` broke `share` of its full strength on a player, thrown from `from`.
pub(crate) fn splashed(world: &mut World, player: Entity, data: i16, share: f32, from: [f32; 3]) {
    if !is_reached(world, player) {
        return;
    }
    for gives in sapling_data::potion_effects(data) {
        let Some(effect) = Effect::ALL.into_iter().find(|effect| effect.name() == gives.effect) else { continue };
        let ticks = gives.ticks as f32 * share;
        if !effect.is_instant() && ticks < SHORTEST {
            continue;
        }
        if effect == Effect::InstantDamage
            && is_mortal(world, player)
            && let Some(id) = world.get::<NetId>(player).map(|id| id.0)
        {
            struck::throw_from(world, player, id, from);
        }
        effects::give(world, player, effect, (ticks + 0.5) as u32, gives.amplifier, true);
    }
}

/// A cloud's pulse of the potion of data value `data` on a player (clouds.rs): `share` of each
/// effect's time and of an instant one's strength; its harm throws as a magic hit by `slayer` from `from`.
pub(crate) fn clouded(world: &mut World, player: Entity, data: i16, [time, instant]: [f32; 2], slayer: &str, from: [f32; 3]) {
    if !is_reached(world, player) {
        return;
    }
    for gives in sapling_data::potion_effects(data) {
        let Some(effect) = Effect::ALL.into_iter().find(|effect| effect.name() == gives.effect) else { continue };
        let strength = 2f32.powi(i32::from(gives.amplifier)) * instant;
        match effect {
            Effect::InstantDamage => drop(struck::fanged(world, player, effects::HARMS * strength, slayer, from)),
            Effect::InstantHealth => {
                if let Some(mut vitals) = world.get_mut::<Vitals>(player).filter(|vitals| vitals.died.is_none()) {
                    vitals.health.mend(effects::HEALS * strength);
                }
            }
            _ => drop(effects::give(world, player, effect, (gives.ticks as f32 * time) as u32, gives.amplifier, true)),
        }
    }
}

/// Whether the effect named `effect` is on the player.
pub(crate) fn is_under(world: &World, player: Entity, effect: &str) -> bool {
    Effect::ALL.into_iter().find(|known| known.name() == effect).is_some_and(|effect| effects::level(world, player, effect).is_some())
}

pub(crate) fn health_left(world: &World, player: Entity) -> f32 {
    world.get::<Vitals>(player).map_or(0.0, |vitals| vitals.health.left())
}
