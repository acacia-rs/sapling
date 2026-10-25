//! Effects on a mob, as BDS's were drilled (research/captures/bds-mob-effects-findings.md): the
//! turns and sizes of a player's (`vitals::effects`), counted down a tick at a time, shown to its
//! viewers in `VisibleMobEffects` and, for speed and slowness alone, a `MobEffect`. An undead mob
//! is mended by instant damage and hurt, unflashed, by instant health, each its own size, and
//! takes no poison or regeneration at all. A mob an effect hurts runs.
// TODO: BDS's flags for fire resistance and water breathing (fire immune, breathing) are not
// shown, nor is speed's change to the movement attribute; one who comes to see a mob is not told its effects, they are not saved
// with it, strength and weakness do not change its hit, and levitation, slow falling and jump
// boost do nothing to it.

use acacia_proto::packets::{MobEffect, MobEffectEventId};
use bevy_ecs::prelude::*;
use sapling_api::Effect;

use super::health::{Cause, Health};
use super::{Creature, Dying, harm, strolling};
use crate::body::Body;
use crate::components::{InLayer, NetId};
use crate::entities;
use crate::vitals::effects::{HARMS, HEALS, POISONED, POISONS, QUICKENS, REGENERATES, SLOWS, WITHERS};

/// What a level of resistance takes off a hit.
const RESISTS: f32 = 0.2;
/// Effects shorter than this, shared out by a splash, are not given.
const SHORTEST: f32 = 20.0;

#[derive(Clone, Copy)]
struct Active {
    effect: Effect,
    amplifier: u8,
    left: u32,
}

#[derive(Component, Default)]
pub(super) struct Afflicted(Vec<Active>);

/// Whether BDS tells a mob's viewers of the effect with a `MobEffect`, as it did speed and slowness alone.
fn is_told(effect: Effect) -> bool {
    matches!(effect, Effect::Speed | Effect::Slowness)
}

fn told(id: u64, event_id: MobEffectEventId, active: &Active) -> bytes::Bytes {
    sapling_net::encode(&MobEffect { runtime_entity_id: id, event_id, effect_id: active.effect.id(), amplifier: i32::from(active.amplifier), particles: true, duration: active.left as i32, tick: 0, ambient: false })
}

fn shown(id: u64, active: &[Active]) -> bytes::Bytes {
    sapling_net::encode(&sapling_data::visible_effects(id, active.iter().map(|active| active.effect.id())))
}

fn tell(world: &mut World, mob: Entity, packets: Vec<bytes::Bytes>) {
    let (Some(layer), Some(at)) = (world.get::<InLayer>(mob).map(|layer| layer.0), world.get::<Body>(mob).map(|body| body.at)) else { return };
    entities::tell_viewers(world, mob, layer, at.map(|c| c.floor() as i32), packets);
}

fn is_undead(world: &World, mob: Entity) -> bool {
    world.get::<entities::Kind>(mob).is_some_and(|kind| sapling_data::is_undead(&kind.0))
}

/// The level, from 0, of an effect on a mob.
pub(super) fn level(world: &World, mob: Entity, effect: Effect) -> Option<u8> {
    world.get::<Afflicted>(mob)?.0.iter().find(|active| active.effect == effect).map(|active| active.amplifier)
}

/// What the mob's walking speed is multiplied by under its speed and slowness (BDS's speed II
/// stroll fit it to 1e-5; slowness 255 stood a zombie still).
pub(super) fn pace(world: &World, mob: Entity) -> f32 {
    let by = |effect: Effect, each: f32| level(world, mob, effect).map_or(1.0, |level| 1.0 + each * (f32::from(level) + 1.0));
    (by(Effect::Speed, QUICKENS) * by(Effect::Slowness, -SLOWS)).max(0.0)
}

/// Gives a living mob an effect, as `ShardCtx::give_effect` gives a player one; an undead one
/// takes no poison or regeneration. Whether it took it.
pub(super) fn give(world: &mut World, mob: Entity, effect: Effect, ticks: u32, amplifier: u8) -> bool {
    if world.get::<Health>(mob).is_none() || world.get::<Dying>(mob).is_some() || (is_undead(world, mob) && matches!(effect, Effect::Poison | Effect::Regeneration)) {
        return false;
    }
    let Some(id) = world.get::<NetId>(mob).map(|id| id.0) else { return false };
    let new = Active { effect, amplifier, left: if effect.is_instant() { 1 } else { ticks } };
    if world.get::<Afflicted>(mob).is_none() {
        world.entity_mut(mob).insert(Afflicted::default());
    }
    let mut afflicted = world.get_mut::<Afflicted>(mob).expect("put above");
    let (event_id, now) = match afflicted.0.iter_mut().find(|active| active.effect == effect) {
        Some(old) => {
            if (new.amplifier, new.left) > (old.amplifier, old.left) {
                *old = new;
            }
            (MobEffectEventId::Update, *old)
        }
        None => {
            afflicted.0.push(new);
            (MobEffectEventId::Add, new)
        }
    };
    let mut packets = vec![shown(id, &afflicted.0)];
    if is_told(effect) {
        packets.push(told(id, event_id, &now));
    }
    tell(world, mob, packets);
    true
}

/// Takes `only` off a mob, or every effect it has: whether it had any of them.
pub(super) fn take(world: &mut World, mob: Entity, only: Option<Effect>) -> bool {
    let Some(mut afflicted) = world.get_mut::<Afflicted>(mob) else { return false };
    let gone: Vec<Active> = afflicted.0.iter().copied().filter(|active| only.is_none_or(|effect| active.effect == effect)).collect();
    afflicted.0.retain(|active| only.is_some_and(|effect| active.effect != effect));
    lose(world, mob, &gone);
    !gone.is_empty()
}

/// Tells a mob's viewers what is left of its effects after `gone` went; one with none left is
/// warded no more.
fn lose(world: &mut World, mob: Entity, gone: &[Active]) {
    let (Some(id), Some(afflicted)) = (world.get::<NetId>(mob).map(|id| id.0), world.get::<Afflicted>(mob)) else { return };
    if gone.is_empty() {
        return;
    }
    let left = afflicted.0.is_empty();
    let mut packets = vec![shown(id, &afflicted.0)];
    packets.extend(gone.iter().filter(|active| is_told(active.effect)).map(|active| told(id, MobEffectEventId::Remove, active)));
    if left
        && let Some(mut health) = world.get_mut::<Health>(mob)
    {
        health.ward(1.0, false);
    }
    tell(world, mob, packets);
}

/// Gives a mob the effect named `effect` at its first level for `ticks`.
pub(super) fn afflict(world: &mut World, mob: Entity, effect: &str, ticks: u32) {
    if let Some(effect) = Effect::ALL.into_iter().find(|known| known.name() == effect) {
        give(world, mob, effect, ticks, 0);
    }
}

/// The potion of data value `data` broke `share` of its full strength on a mob.
pub(super) fn splashed(world: &mut World, mob: Entity, data: i16, share: f32) {
    for gives in sapling_data::potion_effects(data) {
        let Some(effect) = Effect::ALL.into_iter().find(|effect| effect.name() == gives.effect) else { continue };
        let ticks = gives.ticks as f32 * share;
        if effect.is_instant() || ticks >= SHORTEST {
            give(world, mob, effect, (ticks + 0.5) as u32, gives.amplifier);
        }
    }
}

/// A cloud's instant harm on a mob, at `share` of a potion's (the undead are mended instead).
pub(crate) fn breathed_on(world: &mut World, mob: Entity, layer: Entity, share: f32) {
    if is_undead(world, mob) {
        if let Some(mut health) = world.get_mut::<Health>(mob) {
            health.mend(HARMS * share);
        }
        return;
    }
    harm::wound(world, mob, layer, HARMS * share, Cause::Magic);
    strolling::startle(world, mob);
}

/// A tick of one effect on a mob: it turns when what is left of it is a whole number of its
/// turns, its first tick too, and it is gone before 0.
fn act(world: &mut World, mob: Entity, layer: Entity, active: &Active) {
    let undead = is_undead(world, mob);
    let turn = |every: u32| active.left.is_multiple_of((every >> active.amplifier.min(31)).max(1));
    let strength = 2f32.powi(i32::from(active.amplifier));
    let left = world.get::<Health>(mob).map_or(0.0, Health::left);
    let (mend, harm, cause) = match (active.effect, undead) {
        (Effect::InstantHealth, false) => (HEALS * strength, 0.0, Cause::Magic),
        (Effect::InstantDamage, true) => (HARMS * strength, 0.0, Cause::Magic),
        (Effect::InstantDamage, false) => (0.0, HARMS * strength, Cause::Magic),
        (Effect::InstantHealth, true) => {
            // Unflashed and through armour, as BDS hurt its zombie.
            let Some(harm) = crate::damage::ask(world, mob, HEALS * strength, Cause::Magic, None) else { return };
            let dies = world.get_mut::<Health>(mob).is_some_and(|mut health| {
                health.mend(-harm);
                if health.left() <= 0.0 {
                    health.end();
                }
                health.left() <= 0.0
            });
            if dies {
                let (Some(kind), Some(body)) = (world.get::<entities::Kind>(mob).map(|kind| kind.0.clone()), world.get::<Body>(mob).copied()) else { return };
                super::die(world, mob, layer, &kind, &body);
            }
            return;
        }
        (Effect::Regeneration, _) if turn(REGENERATES) => (1.0, 0.0, Cause::Magic),
        (Effect::Poison, _) if turn(POISONS) && left > POISONED => (0.0, 1.0, Cause::Magic),
        (Effect::Wither, _) if turn(WITHERS) => (0.0, 1.0, Cause::Wither),
        _ => (0.0, 0.0, Cause::Magic),
    };
    if mend > 0.0
        && let Some(mut health) = world.get_mut::<Health>(mob)
    {
        health.mend(mend);
    }
    if harm > 0.0 {
        harm::wound(world, mob, layer, harm, cause);
        strolling::startle(world, mob);
    }
}

/// A tick of every mob's effects; those whose time is up go, and its viewers are told what is left.
pub(super) fn tick(world: &mut World) {
    let afflicted: Vec<(Entity, Entity, Vec<Active>)> = world
        .query_filtered::<(Entity, &InLayer, &Afflicted), (With<Creature>, Without<Dying>)>()
        .iter(world)
        .filter(|(.., afflicted)| !afflicted.0.is_empty())
        .map(|(mob, layer, afflicted)| (mob, layer.0, afflicted.0.clone()))
        .collect();
    for (mob, layer, active) in afflicted {
        for active in &active {
            act(world, mob, layer, active);
        }
        let resists = level(world, mob, Effect::Resistance).map_or(0.0, |level| RESISTS * (f32::from(level) + 1.0));
        let fireproof = level(world, mob, Effect::FireResistance).is_some();
        if let Some(mut health) = world.get_mut::<Health>(mob) {
            health.ward((1.0 - resists).max(0.0), fireproof);
        }
        let Some(mut afflicted) = world.get_mut::<Afflicted>(mob) else { continue };
        afflicted.0.iter_mut().for_each(|active| active.left = active.left.saturating_sub(1));
        let gone: Vec<Active> = afflicted.0.iter().copied().filter(|active| active.left == 0).collect();
        afflicted.0.retain(|active| active.left > 0);
        lose(world, mob, &gone);
    }
}
