//! Potions on a mob: a splash's share, a cloud's pulses, whether a cloud reaches it at all.

use bevy_ecs::prelude::*;
use sapling_api::Effect;

use super::{give, is_undead, refuses};
use crate::creatures::health::{Cause, Health};
use crate::creatures::{blows, harm, strolling};
use crate::vitals::effects::{HARMS, HEALS};

/// Effects shorter than this, shared out by a splash, are not given.
const SHORTEST: f32 = 20.0;

/// Whether a cloud of the potion `data` reaches the mob: it takes one of the potion's effects.
pub(crate) fn takes_potion(world: &World, mob: Entity, data: i16) -> bool {
    let mut effects = sapling_data::potion_effects(data).iter().filter_map(|gives| Effect::ALL.into_iter().find(|effect| effect.name() == gives.effect));
    effects.any(|effect| !refuses(world, mob, effect))
}

/// The potion of data value `data` broke `share` of its full strength on a mob.
pub(crate) fn splashed(world: &mut World, mob: Entity, data: i16, share: f32) {
    for gives in sapling_data::potion_effects(data) {
        let Some(effect) = Effect::ALL.into_iter().find(|effect| effect.name() == gives.effect) else { continue };
        let ticks = gives.ticks as f32 * share;
        if effect.is_instant() || ticks >= SHORTEST {
            give(world, mob, effect, (ticks + 0.5) as u32, gives.amplifier);
        }
    }
}

/// A cloud's pulse at `from` of the potion of data value `data` on a mob (clouds.rs): `share` of each
/// effect's time and of an instant one's strength, the undead harmed and mended the other way.
pub(crate) fn clouded(world: &mut World, mob: Entity, layer: Entity, data: i16, [time, instant]: [f32; 2], from: [f32; 3]) {
    for gives in sapling_data::potion_effects(data) {
        let Some(effect) = Effect::ALL.into_iter().find(|effect| effect.name() == gives.effect) else { continue };
        let strength = 2f32.powi(i32::from(gives.amplifier)) * instant;
        match effect {
            Effect::InstantDamage => breathed_on(world, mob, layer, strength, from),
            Effect::InstantHealth if is_undead(world, mob) => drop(harm::wound(world, mob, layer, HEALS * strength, Cause::Magic)),
            Effect::InstantHealth => {
                if let Some(mut health) = world.get_mut::<Health>(mob) {
                    health.mend(HEALS * strength);
                }
            }
            _ => drop(give(world, mob, effect, (gives.ticks as f32 * time) as u32, gives.amplifier)),
        }
    }
}

/// A cloud's instant harm on a mob, at `share` of a potion's, throwing it from the cloud at `from`
/// as a hit (bds-y3arrow.txt: a held cow thrown 2 blocks out, harmed once in the cloud's 30 s); the
/// undead are mended instead.
fn breathed_on(world: &mut World, mob: Entity, layer: Entity, share: f32, from: [f32; 3]) {
    if is_undead(world, mob) {
        if let Some(mut health) = world.get_mut::<Health>(mob) {
            health.mend(HARMS * share);
        }
        return;
    }
    harm::wound(world, mob, layer, HARMS * share, Cause::Magic);
    blows::throw_from(world, mob, from, 0, false);
    strolling::startle(world, mob);
}
