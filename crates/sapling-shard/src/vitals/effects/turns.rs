//! A tick of the effects on every player: what each does in its turn, then the count down.

use bevy_ecs::prelude::*;
use sapling_api::Effect;

use super::super::{Vitals, feed, hurt};
use super::{Active, Effects, HARMS, HEALS, POISONED, POISONS, REGENERATES, WITHERS, take, told};
use crate::components::in_world;
use crate::creatures::Cause;

/// Exhaustion a tick of hunger adds a level.
const HUNGERS: f32 = 0.005;

/// A tick of one effect on a player, `left` ticks of it to go.
fn act(world: &mut World, player: Entity, active: &Active) {
    let levels = f32::from(active.amplifier) + 1.0;
    let turn = |every: u32| active.left.is_multiple_of((every >> active.amplifier.min(31)).max(1));
    let health = world.get::<Vitals>(player).map_or(0.0, |vitals| vitals.health.left());
    match active.effect {
        Effect::InstantHealth | Effect::Regeneration => {
            let by = if active.effect == Effect::Regeneration { f32::from(u8::from(turn(REGENERATES))) } else { HEALS * 2f32.powi(i32::from(active.amplifier)) };
            let by = if by > 0.0 { crate::life_events::healed(world, player, by, if active.effect == Effect::Regeneration { sapling_api::HealSource::Regeneration } else { sapling_api::HealSource::InstantHealth }) } else { by };
            if let Some(mut vitals) = world.get_mut::<Vitals>(player).filter(|vitals| by > 0.0 && vitals.died.is_none()) {
                vitals.health.mend(by);
            }
        }
        Effect::InstantDamage => drop(hurt(world, player, HARMS * 2f32.powi(i32::from(active.amplifier)), Cause::Magic)),
        Effect::Poison if turn(POISONS) && health > POISONED => drop(hurt(world, player, 1.0, Cause::Magic)),
        Effect::FatalPoison if turn(POISONS) => drop(hurt(world, player, 1.0, Cause::Magic)),
        Effect::Wither if turn(WITHERS) => drop(hurt(world, player, 1.0, Cause::Wither)),
        Effect::Hunger => super::super::tire(world, player, HUNGERS * levels),
        // BDS left saturation at 6 while a stew's six ticks fed hunger 6 to 12 (bds-foods2.txt).
        Effect::Saturation => feed(world, player, |hunger| hunger.feed(levels, 0.0)),
        // Resistance, fire resistance, jump boost and slow falling: `vitals::dress`. Water
        // breathing and conduit power: `vitals::stand`. Strength and weakness: `hits_for`.
        _ => {}
    }
}

/// A tick of every player's effects; those whose time is up are told gone.
pub(in crate::vitals) fn tick(world: &mut World) {
    super::invisible::heard(world);
    for player in in_world(world) {
        told::greet(world, player);
        if world.get::<Vitals>(player).is_some_and(|vitals| vitals.died.is_some()) {
            continue;
        }
        let now = world.resource::<crate::components::Tick>().0;
        let Some(active) = world.get::<Effects>(player).map(|effects| effects.0.clone()).filter(|active| !active.is_empty()) else { continue };
        for active in active.iter().filter(|active| active.since != now) {
            act(world, player, active);
        }
        if let Some(mut effects) = world.get_mut::<Effects>(player) {
            effects.0.iter_mut().filter(|active| active.since != now).for_each(|active| active.left = active.left.saturating_sub(1));
            effects.1 = true;
        }
        take(world, player, |active| active.left == 0);
    }
}
