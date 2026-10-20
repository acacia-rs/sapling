//! Effects on a player, counted down a tick at a time and told as BDS tells them
//! (docs/research/effects.md).
// TODO: the effects drilled and no others; one who comes
// to see a player is not told its effects, nor are `VisibleMobEffects`; they end unsaid as the
// player rises. Kept between visits (inventory/store.rs) are the effects and not what is left
// of absorption's cushion, and what BDS tells of them at a join was not drilled.

use acacia_proto::packets::{MobEffect, MobEffectEventId, UpdateAttributes};
use acacia_proto::types::{PlayerAttributesItem, PlayerAttributesItemModifiersItem};
use bevy_ecs::prelude::*;
use sapling_api::{Effect, EffectBackend, PlayerId};

use super::{Vitals, feed, hurt, settle, tell};
use crate::components::{NetId, Tick, in_world};
use crate::creatures::Cause;
use crate::handlers::Ctx;

/// What instant health gives and instant damage takes at the first level; each level doubles it.
const HEALS: f32 = 4.0;
const HARMS: f32 = 6.0;
/// Ticks between two of regeneration's, poison's and wither's turns at the first level; each
/// level halves them.
const REGENERATES: u32 = 50;
const POISONS: u32 = 25;
const WITHERS: u32 = 40;
/// Poison leaves this much health.
const POISONED: f32 = 1.0;
/// Exhaustion a tick of hunger adds a level.
const HUNGERS: f32 = 0.005;
/// A player's walking speed, and what a level of speed adds of it.
const WALKS: f32 = 0.1;
const QUICKENS: f32 = 0.2;
const QUICKENS_AS: &str = "91aeaa56-376b-4498-935b-2f7f68070635";
/// Health a level of health boost adds to the most a player has.
const BOOSTS: f32 = 4.0;
const BOOSTS_AS: &str = "5d6f0ba2-1186-46ac-b896-c61c5cee99cc";
/// What a level of slowness takes of it.
const SLOWS: f32 = 0.15;
const SLOWS_AS: &str = "7107de5e-7ce8-4030-940e-514c1f160890";
/// A level of strength multiplies a hit by the first and adds the second; of weakness likewise
/// (research/captures/bds-fx.txt: a fist's 1 was 3.99 at strength's second level, 0.3 weak).
const STRENGTHENS: [f32; 2] = [1.3, 0.3];
const WEAKENS: [f32; 2] = [0.8, 0.4];
/// Health a level of absorption puts on top, and the most the client shows.
const CUSHIONS: f32 = 4.0;
const MOST_CUSHION: f32 = 16.0;

#[derive(Clone, Copy)]
struct Active {
    effect: Effect,
    amplifier: u8,
    left: u32,
    particles: bool,
}

/// The effects on a player, whether storage has yet to hear of them, and whether the player,
/// back with them, has yet to.
#[derive(Component, Default)]
pub(crate) struct Effects(Vec<Active>, bool, bool);

/// What storage keeps of a player's effects (`sapling_leveldb::PlayerData::effects`), if they
/// changed since it last asked.
pub(crate) fn kept(world: &mut World, player: Entity) -> Option<Vec<[i32; 4]>> {
    let mut effects = world.get_mut::<Effects>(player).filter(|effects| effects.1)?;
    effects.1 = false;
    Some(effects.0.iter().map(|active| [active.effect.id(), i32::from(active.amplifier), active.left as i32, i32::from(active.particles)]).collect())
}

/// The effects storage kept, for a player coming back.
pub(crate) fn restored(kept: &[[i32; 4]]) -> Option<Effects> {
    let active = |[id, amplifier, left, shown]: &[i32; 4]| Some(Active { effect: Effect::ALL.into_iter().find(|known| known.id() == *id)?, amplifier: *amplifier as u8, left: u32::try_from(*left).ok()?, particles: *shown != 0 });
    let active: Vec<Active> = kept.iter().filter_map(active).collect();
    (!active.is_empty()).then(|| Effects(active, false, true))
}

/// Tells a player in the world the effects it came back with, once.
fn greet(world: &mut World, player: Entity) {
    let (Some(id), Some(mut effects)) = (world.get::<NetId>(player).map(|id| id.0), world.get_mut::<Effects>(player).filter(|effects| effects.2)) else { return };
    effects.2 = false;
    for active in effects.0.clone() {
        tell(world, player, &told(id, MobEffectEventId::Add, &active, active.left));
        match active.effect {
            Effect::Speed | Effect::Slowness => tell_pace(world, player, id),
            Effect::HealthBoost => boost(world, player, id),
            _ => {}
        }
    }
}

fn told(id: u64, event_id: MobEffectEventId, active: &Active, duration: u32) -> MobEffect {
    MobEffect { runtime_entity_id: id, event_id, effect_id: active.effect.id(), amplifier: i32::from(active.amplifier), particles: active.particles, duration: duration as i32, tick: 0, ambient: false }
}

/// The player's walking speed under levels of speed and of slowness, as BDS writes it.
// TODO: both at once were not drilled: each is taken to multiply the other's.
fn pace(id: u64, quick: Option<u8>, slow: Option<u8>, tick: u64) -> UpdateAttributes {
    let modifier = |levels: Option<u8>, id: &str, name: &str, each: f32| {
        levels.map(|levels| PlayerAttributesItemModifiersItem { id: id.into(), name: name.into(), amount: each * (f32::from(levels) + 1.0), operation: 2, operand: 2, serializable: false })
    };
    let modifiers: Vec<_> = [modifier(quick, QUICKENS_AS, "MovementSpeed", QUICKENS), modifier(slow, SLOWS_AS, "MovementSlowdown", -SLOWS)].into_iter().flatten().collect();
    let movement = PlayerAttributesItem {
        min: 0.0,
        max: f32::MAX,
        current: modifiers.iter().fold(WALKS, |pace, modifier| pace * (1.0 + modifier.amount)).max(0.0),
        default_min: 0.0,
        default_max: f32::MAX,
        default: WALKS,
        name: "minecraft:movement".into(),
        modifiers,
    };
    UpdateAttributes { runtime_entity_id: id, attributes: vec![movement], tick }
}

/// The health on top of a player's own, as BDS writes it.
pub(super) fn cushion(id: u64, amount: f32, tick: u64) -> UpdateAttributes {
    let absorption = PlayerAttributesItem { min: 0.0, max: MOST_CUSHION, current: amount, default_min: 0.0, default_max: MOST_CUSHION, default: 0.0, name: "minecraft:absorption".into(), modifiers: Vec::new() };
    UpdateAttributes { runtime_entity_id: id, attributes: vec![absorption], tick }
}

/// Puts `amount` of health on top of a player's own, told at once.
fn pad(world: &mut World, player: Entity, amount: f32) {
    if let Some(mut vitals) = world.get_mut::<Vitals>(player) {
        vitals.health.pad(amount);
    }
    settle(world, player, Cause::Generic);
}

/// Sets the most health a player has under the health boost on it now, and tells it.
fn boost(world: &mut World, player: Entity, id: u64) {
    let more = level(world, player, Effect::HealthBoost).map(|level| BOOSTS * (f32::from(level) + 1.0));
    let Some(mut vitals) = world.get_mut::<Vitals>(player) else { return };
    vitals.health.grow(super::MAX + more.unwrap_or(0.0));
    let mut told = sapling_data::mob_health_update(id, vitals.health.left(), vitals.health.max());
    (told.attributes[0].default, told.attributes[0].default_max) = (super::MAX, super::MAX);
    told.attributes[0].modifiers = more.map(|amount| PlayerAttributesItemModifiersItem { id: BOOSTS_AS.into(), name: "HealthBoost".into(), amount, operation: 0, operand: 1, serializable: false }).into_iter().collect();
    tell(world, player, &told);
}

/// Tells a player its walking speed under the effects on it now.
fn tell_pace(world: &mut World, player: Entity, id: u64) {
    let tick = world.resource::<Tick>().0;
    let (quick, slow) = (level(world, player, Effect::Speed), level(world, player, Effect::Slowness));
    let told = pace(id, quick, slow, tick);
    if let Some(mut simulated) = world.get_mut::<crate::simulation::Simulated>(player) {
        simulated.set_speed(told.attributes[0].current);
    }
    tell(world, player, &told);
}

/// What a hit of `amount` by a player costs under its strength and its weakness.
pub(crate) fn hits_for(world: &World, player: Entity, amount: f32) -> f32 {
    let under = |amount: f32, effect: Effect, [times, plus]: [f32; 2]| match level(world, player, effect) {
        Some(level) => {
            let by = times.powi(i32::from(level) + 1);
            amount * by + (by - 1.0) / plus
        }
        None => amount,
    };
    under(under(amount, Effect::Strength, STRENGTHENS), Effect::Weakness, WEAKENS).max(0.0)
}

/// Takes the effects `gone` picks off a player, telling each with what was left of it.
fn take(world: &mut World, player: Entity, gone: impl Fn(&Active) -> bool) -> bool {
    let (Some(id), Some(mut effects)) = (world.get::<NetId>(player).map(|id| id.0), world.get_mut::<Effects>(player)) else { return false };
    let taken: Vec<Active> = effects.0.iter().copied().filter(|active| gone(active)).collect();
    effects.0.retain(|active| !gone(active));
    effects.1 |= !taken.is_empty();
    for active in &taken {
        tell(world, player, &told(id, MobEffectEventId::Remove, active, active.left));
        match active.effect {
            Effect::Speed | Effect::Slowness => tell_pace(world, player, id),
            Effect::Absorption => pad(world, player, 0.0),
            Effect::HealthBoost => boost(world, player, id),
            _ => {}
        }
    }
    !taken.is_empty()
}

/// The level, from 0, of an effect on a player.
pub(super) fn level(world: &World, player: Entity, effect: Effect) -> Option<u8> {
    world.get::<Effects>(player)?.0.iter().find(|active| active.effect == effect).map(|active| active.amplifier)
}

/// Takes one effect off a player; false where it was not on it.
pub(super) fn end(world: &mut World, player: Entity, effect: Effect) -> bool {
    take(world, player, |active| active.effect == effect)
}

/// Takes every effect off a player; false where it had none.
pub(super) fn clear(world: &mut World, player: Entity) -> bool {
    take(world, player, |_| true)
}

/// See `ShardCtx::give_effect`.
pub(super) fn give(world: &mut World, player: Entity, effect: Effect, ticks: u32, amplifier: u8, particles: bool) -> bool {
    let Some(id) = world.get::<NetId>(player).map(|id| id.0) else { return false };
    let new = Active { effect, amplifier, left: if effect.is_instant() { 1 } else { ticks }, particles };
    if world.get::<Effects>(player).is_none() {
        world.entity_mut(player).insert(Effects::default());
    }
    let mut effects = world.get_mut::<Effects>(player).expect("put above");
    let (event_id, now) = match effects.0.iter_mut().find(|active| active.effect == effect) {
        Some(old) => {
            if (new.amplifier, new.left) > (old.amplifier, old.left) {
                *old = new;
            }
            (MobEffectEventId::Update, *old)
        }
        None => {
            effects.0.push(new);
            (MobEffectEventId::Add, new)
        }
    };
    effects.1 = true;
    tell(world, player, &told(id, event_id, &now, now.left));
    match effect {
        Effect::Speed | Effect::Slowness => tell_pace(world, player, id),
        Effect::Absorption => pad(world, player, CUSHIONS * (f32::from(now.amplifier) + 1.0)),
        Effect::HealthBoost => boost(world, player, id),
        _ => {}
    }
    true
}

/// Gives a living player the effect named `effect` at its first level for `ticks`.
pub(crate) fn afflict(world: &mut World, player: Entity, effect: &str, ticks: u32) {
    if let Some(effect) = Effect::ALL.into_iter().find(|known| known.name() == effect) {
        give(world, player, effect, ticks, 0, true);
    }
}

/// A tick of one effect on a player, `left` ticks of it to go.
fn act(world: &mut World, player: Entity, active: &Active) {
    let levels = f32::from(active.amplifier) + 1.0;
    let turn = |every: u32| active.left.is_multiple_of((every >> active.amplifier.min(31)).max(1));
    let health = world.get::<Vitals>(player).map_or(0.0, |vitals| vitals.health.left());
    match active.effect {
        Effect::InstantHealth | Effect::Regeneration => {
            let by = if active.effect == Effect::Regeneration { f32::from(u8::from(turn(REGENERATES))) } else { HEALS * 2f32.powi(i32::from(active.amplifier)) };
            if let Some(mut vitals) = world.get_mut::<Vitals>(player).filter(|vitals| by > 0.0 && vitals.died.is_none()) {
                vitals.health.mend(by);
            }
        }
        Effect::InstantDamage => drop(hurt(world, player, HARMS * 2f32.powi(i32::from(active.amplifier)), Cause::Magic)),
        Effect::Poison if turn(POISONS) && health > POISONED => drop(hurt(world, player, 1.0, Cause::Magic)),
        Effect::Wither if turn(WITHERS) => drop(hurt(world, player, 1.0, Cause::Wither)),
        Effect::Hunger => feed(world, player, |hunger| hunger.tire(HUNGERS * levels)),
        Effect::Saturation => feed(world, player, |hunger| hunger.feed(levels, levels * 2.0)),
        // Resistance, fire resistance, jump boost and slow falling: `vitals::dress`. Water
        // breathing: `vitals::tick`. Strength and weakness: `hits_for`.
        _ => {}
    }
}

/// A tick of every player's effects; those whose time is up are told gone.
pub(super) fn tick(world: &mut World) {
    for player in in_world(world) {
        greet(world, player);
        let Some(active) = world.get::<Effects>(player).map(|effects| effects.0.clone()).filter(|active| !active.is_empty()) else { continue };
        for active in &active {
            act(world, player, active);
        }
        if let Some(mut effects) = world.get_mut::<Effects>(player) {
            effects.0.iter_mut().for_each(|active| active.left = active.left.saturating_sub(1));
            effects.1 = true;
        }
        take(world, player, |active| active.left == 0);
    }
}

impl EffectBackend for Ctx<'_> {
    fn give_effect(&mut self, player: PlayerId, effect: Effect, ticks: u32, amplifier: u8, particles: bool) -> bool {
        self.entity(player).is_some_and(|entity| give(self.world, entity, effect, ticks, amplifier, particles))
    }

    fn remove_effect(&mut self, player: PlayerId, effect: Effect) -> bool {
        self.entity(player).is_some_and(|entity| end(self.world, entity, effect))
    }

    fn clear_effects(&mut self, player: PlayerId) -> bool {
        self.entity(player).is_some_and(|entity| clear(self.world, entity))
    }
}
