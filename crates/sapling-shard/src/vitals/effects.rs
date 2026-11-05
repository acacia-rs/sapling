//! Effects on a player, counted down a tick at a time and told as BDS tells them
//! (docs/research/effects.md).
// TODO: the effects drilled and no others; one who comes to see a player is told its
// `VisibleMobEffects` (`announced`) and, as on BDS, no `MobEffect`. Kept between visits (inventory/store.rs) are the effects and not what is left
// of absorption's cushion, and what BDS tells of them at a join was not drilled.

mod attributes;
mod combat;

use acacia_proto::packets::{MobEffect, MobEffectEventId};
pub(crate) use attributes::{Paced, cushion, tell_pace, walking};
pub(crate) use combat::{dig_speed, hits_for};
use bevy_ecs::prelude::*;
use sapling_api::{Effect, EffectBackend, PlayerId};

use super::{Vitals, feed, hurt, settle, tell};
use crate::components::{NetId, in_world};
use crate::creatures::Cause;
use crate::handlers::Ctx;

/// What instant health gives and instant damage takes at the first level; each level doubles it.
pub(crate) const HEALS: f32 = 4.0;
pub(crate) const HARMS: f32 = 6.0;
/// Ticks between two of regeneration's, poison's and wither's turns at the first level; each
/// level halves them.
pub(crate) const REGENERATES: u32 = 50;
pub(crate) const POISONS: u32 = 25;
pub(crate) const WITHERS: u32 = 40;
/// Poison leaves this much health.
pub(crate) const POISONED: f32 = 1.0;
/// Exhaustion a tick of hunger adds a level.
const HUNGERS: f32 = 0.005;
/// What a level of speed adds of a player's walking speed, and a level of slowness takes.
pub(crate) const QUICKENS: f32 = 0.2;
pub(crate) const SLOWS: f32 = 0.15;
/// Health a level of absorption puts on top.
const CUSHIONS: f32 = 4.0;

#[derive(Clone, Copy)]
struct Active {
    effect: Effect,
    amplifier: u8,
    left: u32,
    particles: bool,
    /// From a beacon: BDS told it `ambient` (bds-stations.txt). Not kept.
    ambient: bool,
    /// The tick it was given: BDS first counts it the tick after (bds-r1fx.txt: instant health
    /// gone a tick after its `Add`, cleared ones one tick fuller than sapling's were).
    since: u64,
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
    let active = |[id, amplifier, left, shown]: &[i32; 4]| Some(Active { effect: Effect::ALL.into_iter().find(|known| known.id() == *id)?, amplifier: *amplifier as u8, left: u32::try_from(*left).ok()?, particles: *shown != 0, ambient: false, since: 0 });
    let active: Vec<Active> = kept.iter().filter_map(active).collect();
    (!active.is_empty()).then(|| Effects(active, false, true))
}

/// Tells a player in the world the effects it came back with, once.
fn greet(world: &mut World, player: Entity) {
    let (Some(id), Some(mut effects)) = (world.get::<NetId>(player).map(|id| id.0), world.get_mut::<Effects>(player).filter(|effects| effects.2)) else { return };
    effects.2 = false;
    for active in effects.0.clone() {
        tell_effect(world, player, id, MobEffectEventId::Add, &active);
        match active.effect {
            Effect::Speed | Effect::Slowness => tell_pace(world, player, id),
            Effect::HealthBoost => boost(world, player, id),
            _ => {}
        }
    }
}

/// Tells the player of one effect, stamped with the tick it is told in as BDS's (bds-r1buckets.txt).
fn tell_effect(world: &mut World, player: Entity, id: u64, event_id: MobEffectEventId, active: &Active) {
    let tick = world.resource::<crate::components::Tick>().0;
    let told = MobEffect { runtime_entity_id: id, event_id, effect_id: active.effect.id(), amplifier: i32::from(active.amplifier), particles: active.particles, duration: active.left as i32, tick, ambient: active.ambient };
    tell(world, player, &told);
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
    let boosted = level(world, player, Effect::HealthBoost);
    let Some(mut vitals) = world.get_mut::<Vitals>(player) else { return };
    vitals.health.grow(super::boost::boosted_max(super::MAX, boosted));
    let told = super::boost::boosted_health(id, vitals.health.left(), super::MAX, boosted);
    tell(world, player, &told);
}

/// The ids of the effects on a player that show their particles, in order.
fn shown(world: &World, player: Entity) -> Vec<i32> {
    shown_of(world.get::<Effects>(player))
}

fn shown_of(effects: Option<&Effects>) -> Vec<i32> {
    let mut ids: Vec<i32> = effects.map_or_else(Vec::new, |effects| effects.0.iter().filter(|active| active.particles).map(|active| active.effect.id()).collect());
    ids.sort_unstable();
    ids
}

/// The `VisibleMobEffects` a player's AddPlayer carries: BDS announced one under jump boost
/// with 16 (research/captures/bds-lateeffects.txt).
pub(crate) fn announced(id: u64, effects: Option<&Effects>) -> acacia_proto::types::MetadataDictionaryItem {
    sapling_data::visible_effects(id, shown_of(effects)).metadata.remove(0)
}

/// Tells the player and its viewers the effects it shows, where they are not `before`'s.
fn show(world: &mut World, player: Entity, id: u64, before: &[i32]) {
    let now = shown(world, player);
    if now != before {
        tell(world, player, &sapling_data::visible_effects(id, now));
    }
}

/// Takes the effects `gone` picks off a player, telling each with what was left of it.
fn take(world: &mut World, player: Entity, gone: impl Fn(&Active) -> bool) -> bool {
    let before = shown(world, player);
    let (Some(id), Some(mut effects)) = (world.get::<NetId>(player).map(|id| id.0), world.get_mut::<Effects>(player)) else { return false };
    let mut taken: Vec<Active> = effects.0.iter().copied().filter(|active| gone(active)).collect();
    // BDS keeps effects by id, so a clear tells them in id order (bds-q3witch.txt).
    taken.sort_by_key(|active| active.effect.id());
    effects.0.retain(|active| !gone(active));
    effects.1 |= !taken.is_empty();
    for active in &taken {
        tell_effect(world, player, id, MobEffectEventId::Remove, active);
        match active.effect {
            Effect::Speed | Effect::Slowness => tell_pace(world, player, id),
            Effect::Absorption => pad(world, player, 0.0),
            Effect::HealthBoost => boost(world, player, id),
            _ => {}
        }
    }
    show(world, player, id, &before);
    !taken.is_empty()
}

/// The level, from 0, of an effect on a player.
pub(super) fn level(world: &World, player: Entity, effect: Effect) -> Option<u8> {
    world.get::<Effects>(player)?.0.iter().find(|active| active.effect == effect).map(|active| active.amplifier)
}

/// Takes one effect off a player; false where it was not on it.
pub(crate) fn end(world: &mut World, player: Entity, effect: Effect) -> bool {
    take(world, player, |active| active.effect == effect)
}

/// Takes every effect off a player; false where it had none.
pub(super) fn clear(world: &mut World, player: Entity) -> bool {
    take(world, player, |_| true)
}

/// Takes every effect off a player rising, told by id with the ticks left at its death, as BDS
/// told them after its Respawn (bds-r1fx.txt); they stood still while it was dead.
pub(super) fn end_at_rise(world: &mut World, player: Entity) {
    if let Some(mut effects) = world.get_mut::<Effects>(player) {
        effects.0.sort_by_key(|active| active.effect.id());
    }
    clear(world, player);
    world.entity_mut(player).remove::<Effects>();
}

/// See `ShardCtx::give_effect`.
pub(crate) fn give(world: &mut World, player: Entity, effect: Effect, ticks: u32, amplifier: u8, particles: bool) -> bool {
    give_as(world, player, Active { effect, amplifier, left: ticks, particles, ambient: false, since: 0 })
}

/// A beacon's effect: as [`give`], told `ambient`.
pub(crate) fn give_ambient(world: &mut World, player: Entity, effect: Effect, ticks: u32, amplifier: u8) -> bool {
    give_as(world, player, Active { effect, amplifier, left: ticks, particles: true, ambient: true, since: 0 })
}

fn give_as(world: &mut World, player: Entity, new: Active) -> bool {
    let effect = new.effect;
    let Some(id) = world.get::<NetId>(player).map(|id| id.0) else { return false };
    // bds-fx2.txt: darkness came without particles though the command asked for them.
    // A stew's saturation feeds every tick it lasts (bds-foods2.txt); /effect gives it one.
    let since = world.resource::<crate::components::Tick>().0;
    let new = Active { left: if effect.is_instant() && effect != Effect::Saturation { 1 } else { new.left }, particles: new.particles && effect != Effect::Darkness, since, ..new };
    let before = shown(world, player);
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
    tell_effect(world, player, id, event_id, &now);
    match effect {
        Effect::Speed | Effect::Slowness => tell_pace(world, player, id),
        Effect::Absorption => pad(world, player, CUSHIONS * (f32::from(now.amplifier) + 1.0)),
        Effect::HealthBoost => boost(world, player, id),
        _ => {}
    }
    show(world, player, id, &before);
    true
}

/// The amplifier of `effect` on a player and the ticks it has left, if it is on it.
pub(crate) fn on(world: &World, player: Entity, effect: Effect) -> Option<(u8, u32)> {
    world.get::<Effects>(player)?.0.iter().find(|active| active.effect == effect).map(|active| (active.amplifier, active.left))
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
            let by = if by > 0.0 { crate::life_events::healed(world, player, by, if active.effect == Effect::Regeneration { sapling_api::HealSource::Regeneration } else { sapling_api::HealSource::InstantHealth }) } else { by };
            if let Some(mut vitals) = world.get_mut::<Vitals>(player).filter(|vitals| by > 0.0 && vitals.died.is_none()) {
                vitals.health.mend(by);
            }
        }
        Effect::InstantDamage => drop(hurt(world, player, HARMS * 2f32.powi(i32::from(active.amplifier)), Cause::Magic)),
        Effect::Poison if turn(POISONS) && health > POISONED => drop(hurt(world, player, 1.0, Cause::Magic)),
        Effect::FatalPoison if turn(POISONS) => drop(hurt(world, player, 1.0, Cause::Magic)),
        Effect::Wither if turn(WITHERS) => drop(hurt(world, player, 1.0, Cause::Wither)),
        Effect::Hunger => super::tire(world, player, HUNGERS * levels),
        // BDS left saturation at 6 while a stew's six ticks fed hunger 6 to 12 (bds-foods2.txt).
        Effect::Saturation => feed(world, player, |hunger| hunger.feed(levels, 0.0)),
        // Resistance, fire resistance, jump boost and slow falling: `vitals::dress`. Water
        // breathing and conduit power: `vitals::stand`. Strength and weakness: `hits_for`.
        _ => {}
    }
}

/// A tick of every player's effects; those whose time is up are told gone.
pub(super) fn tick(world: &mut World) {
    for player in in_world(world) {
        greet(world, player);
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
