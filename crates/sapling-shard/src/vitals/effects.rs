//! Effects on a player, counted down a tick at a time and told as BDS tells them
//! (docs/research/effects.md).
// TODO: the nine effects drilled and no others; one who comes
// to see a player is not told its effects, nor are `VisibleMobEffects`; they end unsaid as the
// player rises.

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

#[derive(Component, Default)]
pub(crate) struct Effects(Vec<Active>);

fn told(id: u64, event_id: MobEffectEventId, active: &Active, duration: u32) -> MobEffect {
    MobEffect { runtime_entity_id: id, event_id, effect_id: active.effect.id(), amplifier: i32::from(active.amplifier), particles: active.particles, duration: duration as i32, tick: 0, ambient: false }
}

/// The player's walking speed under `levels` of speed, as BDS writes it.
fn pace(id: u64, levels: Option<u8>, tick: u64) -> UpdateAttributes {
    let amount = levels.map_or(0.0, |levels| QUICKENS * (f32::from(levels) + 1.0));
    let modifier = PlayerAttributesItemModifiersItem { id: QUICKENS_AS.into(), name: "MovementSpeed".into(), amount, operation: 2, operand: 2, serializable: false };
    let movement = PlayerAttributesItem {
        min: 0.0,
        max: f32::MAX,
        current: WALKS * (1.0 + amount),
        default_min: 0.0,
        default_max: f32::MAX,
        default: WALKS,
        name: "minecraft:movement".into(),
        modifiers: levels.map(|_| modifier).into_iter().collect(),
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

fn tell_pace(world: &mut World, player: Entity, id: u64, levels: Option<u8>) {
    let tick = world.resource::<Tick>().0;
    tell(world, player, &pace(id, levels, tick));
}

/// Takes the effects `gone` picks off a player, telling each with what was left of it.
fn take(world: &mut World, player: Entity, gone: impl Fn(&Active) -> bool) -> bool {
    let (Some(id), Some(mut effects)) = (world.get::<NetId>(player).map(|id| id.0), world.get_mut::<Effects>(player)) else { return false };
    let taken: Vec<Active> = effects.0.iter().copied().filter(|active| gone(active)).collect();
    effects.0.retain(|active| !gone(active));
    for active in &taken {
        tell(world, player, &told(id, MobEffectEventId::Remove, active, active.left));
        match active.effect {
            Effect::Speed => tell_pace(world, player, id, None),
            Effect::Absorption => pad(world, player, 0.0),
            _ => {}
        }
    }
    !taken.is_empty()
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
    tell(world, player, &told(id, event_id, &now, now.left));
    match effect {
        Effect::Speed => tell_pace(world, player, id, Some(now.amplifier)),
        Effect::Absorption => pad(world, player, CUSHIONS * (f32::from(now.amplifier) + 1.0)),
        _ => {}
    }
    true
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
        Effect::Poison | Effect::Wither | Effect::Speed | Effect::Absorption => {}
    }
}

/// A tick of every player's effects; those whose time is up are told gone.
pub(super) fn tick(world: &mut World) {
    for player in in_world(world) {
        let Some(active) = world.get::<Effects>(player).map(|effects| effects.0.clone()).filter(|active| !active.is_empty()) else { continue };
        for active in &active {
            act(world, player, active);
        }
        if let Some(mut effects) = world.get_mut::<Effects>(player) {
            effects.0.iter_mut().for_each(|active| active.left = active.left.saturating_sub(1));
        }
        take(world, player, |active| active.left == 0);
    }
}

impl EffectBackend for Ctx<'_> {
    fn give_effect(&mut self, player: PlayerId, effect: Effect, ticks: u32, amplifier: u8, particles: bool) -> bool {
        self.entity(player).is_some_and(|entity| give(self.world, entity, effect, ticks, amplifier, particles))
    }

    fn remove_effect(&mut self, player: PlayerId, effect: Effect) -> bool {
        self.entity(player).is_some_and(|entity| take(self.world, entity, |active| active.effect == effect))
    }

    fn clear_effects(&mut self, player: PlayerId) -> bool {
        self.entity(player).is_some_and(|entity| clear(self.world, entity))
    }
}
