//! Effects on a player, counted down a tick at a time (turns.rs) and told as BDS tells them
//! (told.rs; docs/research/effects.md).
// TODO: the effects drilled and no others; one who comes to see a player is told its
// `VisibleMobEffects` (`announced`) and, as on BDS, no `MobEffect`. Kept between visits (inventory/store.rs) are the effects and not what is left
// of absorption's cushion, and what BDS tells of them at a join was not drilled.

mod attributes;
mod combat;
mod invisible;
mod told;
mod turns;

use acacia_proto::packets::MobEffectEventId;
pub(crate) use attributes::{Paced, cushion, mob_walking, tell_pace, walking};
pub(crate) use combat::{dig_speed, hit_under, hits_for};
use bevy_ecs::prelude::*;
use sapling_api::{Effect, EffectBackend, PlayerId};
pub(crate) use told::announced;
use told::{boost, pad, show, shown, tell_effect};
pub(super) use turns::tick;

use crate::components::NetId;
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

/// See `ShardCtx::give_effect`; an omen taking hold is heard ([`omen_sound`]).
pub(crate) fn give(world: &mut World, player: Entity, effect: Effect, ticks: u32, amplifier: u8, particles: bool) -> bool {
    give_as(world, player, Active { effect, amplifier, left: ticks, particles, ambient: false, since: 0 }, true)
}

/// [`give`] with no omen heard: the caller plays it.
pub(crate) fn give_unheard(world: &mut World, player: Entity, effect: Effect, ticks: u32, amplifier: u8, particles: bool) -> bool {
    give_as(world, player, Active { effect, amplifier, left: ticks, particles, ambient: false, since: 0 }, false)
}

/// A beacon's effect: as [`give`], told `ambient`.
pub(crate) fn give_ambient(world: &mut World, player: Entity, effect: Effect, ticks: u32, amplifier: u8) -> bool {
    give_as(world, player, Active { effect, amplifier, left: ticks, particles: true, ambient: true, since: 0 }, true)
}

/// The sound an omen makes as it takes hold of a player or mob, not as it is raised again
/// (research/captures/bds-bd2event.txt).
pub(crate) fn omen_sound(effect: Effect) -> Option<&'static str> {
    match effect {
        Effect::BadOmen => Some("apply_effect.bad_omen"),
        Effect::RaidOmen => Some("apply_effect.raid_omen"),
        Effect::TrialOmen => Some("apply_effect.trial_omen"),
        _ => None,
    }
}

/// Whether an effect given with `asked` particles shows them: darkness (bds-fx2.txt) and bad omen
/// (bds-bd2event.txt, by `/effect` or a bottle) never do.
fn has_particles(effect: Effect, asked: bool) -> bool {
    asked && !matches!(effect, Effect::Darkness | Effect::BadOmen)
}

fn give_as(world: &mut World, player: Entity, new: Active, heard: bool) -> bool {
    let effect = new.effect;
    let Some(id) = world.get::<NetId>(player).map(|id| id.0) else { return false };
    // A stew's saturation feeds every tick it lasts (bds-foods2.txt); /effect gives it one.
    let since = world.resource::<crate::components::Tick>().0;
    let new = Active { left: if effect.is_instant() && effect != Effect::Saturation { 1 } else { new.left }, particles: has_particles(effect, new.particles), since, ..new };
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
    if let (true, MobEffectEventId::Add, Some(sound), Some(position)) = (heard, event_id, omen_sound(effect), world.get::<crate::components::Position>(player)) {
        let position = super::eating::sipped_at(position);
        let heard = acacia_proto::packets::LevelSoundEvent { sound_id: sound.into(), position, extra_data: -1, entity_type: "minecraft:player".into(), is_baby_mob: false, is_global: false, entity_unique_id: id as i64, fire_at_position: None };
        super::tell(world, player, &heard);
    }
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
