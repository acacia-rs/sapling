//! A thrown potion breaking by a player (docs/research/mob-kinds.md, "The witch"): a direct hit
//! gave the potion's effects in full, and harming threw the player as a hit does. An instant
//! effect's strength is shared as its time is, rounded, and comes with no `MobEffect`
//! (research/captures/bds-t2esplash.txt; BDS's thrown_potion_effect 14240ede0).

use bevy_ecs::prelude::*;
use sapling_api::Effect;

use super::{Vitals, effects, is_dead, is_mortal, struck};
use crate::components::{Mode, NetId};

/// A creative player is given a potion's effects too (bds-clouds.txt: the watcher's poison), only
/// not hurt; a spectator nothing.
fn is_reached(world: &World, player: Entity) -> bool {
    !is_dead(world, player) && world.get::<Mode>(player).is_some_and(|mode| mode.0 != sapling_api::GameMode::Spectator)
}

/// Effects of this or fewer ticks, shared out, are not given.
const SHORTEST: u32 = 20;

/// What `share` of an instant effect of `base` at `amplifier` heals or takes, rounded as BDS did.
pub(crate) fn shared_strength(base: f32, amplifier: u8, share: f32) -> f32 {
    ((base * 2f32.powi(i32::from(amplifier)) * share + 0.5) as i32) as f32
}

/// The potion of data value `data` broke `share` of its full strength on a player, thrown from `from`.
pub(crate) fn splashed(world: &mut World, player: Entity, data: i16, share: f32, from: [f32; 3]) {
    if !is_reached(world, player) {
        return;
    }
    for gives in sapling_data::potion_effects(data) {
        let Some(effect) = Effect::ALL.into_iter().find(|effect| effect.name() == gives.effect) else { continue };
        let ticks = gives.ticks as f32 * share;
        if effect == Effect::InstantDamage
            && is_mortal(world, player)
            && let Some(id) = world.get::<NetId>(player).map(|id| id.0)
        {
            struck::throw_from(world, player, id, from);
        }
        let strength = |base: f32| shared_strength(base, gives.amplifier, share);
        match effect {
            Effect::InstantDamage => drop(super::hurt(world, player, strength(effects::HARMS), crate::creatures::Cause::Magic)),
            Effect::InstantHealth => heal(world, player, strength(effects::HEALS)),
            _ if ((ticks + 0.5) as u32) <= SHORTEST => {}
            _ => drop(effects::give(world, player, effect, (ticks + 0.5) as u32, gives.amplifier, true)),
        }
    }
}

fn heal(world: &mut World, player: Entity, amount: f32) {
    let amount = crate::life_events::healed(world, player, amount, sapling_api::HealSource::InstantHealth);
    if let Some(mut vitals) = world.get_mut::<Vitals>(player).filter(|vitals| amount > 0.0 && vitals.died.is_none()) {
        vitals.health.mend(amount);
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
