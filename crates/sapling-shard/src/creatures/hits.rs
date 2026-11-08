//! What a hurting shot does to the player or mob it meets (flight.rs): it costs what
//! `sapling_data::shots` reckons of its speed and a player's Power and draw, throws as a hit does,
//! further by Punch, gives its archer's effect or its tip's potion, and sets alight what Flame or a
//! fireball burns. A mob a player's shot kills is the player's. One that pierces flies on; a
//! trident bounces off and hurts nothing more.

use acacia_proto::packets::SetEntityMotion;
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_data::{enchant, shots};

use super::flight::{self, Loosed};
use super::health::{Cause, Health};
use super::{afflictions, harm, hunting};
use crate::blocks::Dice;
use crate::body::{Body, Liquid};
use crate::components::{NetId, Position, Tick};
use crate::{entities, late_sounds, vitals};

/// A shot throws the mob it hits this fast along its way across and up, a dispenser's arrow as a
/// player's (research/captures/bds-n3knock.txt: (0.399, 0.4, 0.029) along an arrow's way).
const THROW: f32 = 0.4;
const PLAYER_HEIGHT: f32 = 1.8;

/// What becomes of a shot that hit.
pub(super) enum After {
    Gone,
    /// It flies on through (Piercing).
    On,
    /// It bounces back off (a trident).
    Back,
    /// A shield turned it away, a tenth as fast (research/captures/bds-shield.txt).
    Deflected,
    /// It took nothing off whom it met and bounces back a tenth as fast, staying where it met it this tick.
    Bounced,
}

/// An arrow that hit a body: it goes the tick after, without moving, as BDS's, so one that hit in
/// the tick it was loosed is still announced (research/captures/bds-shoot.txt, bds-n3knock.txt).
/// Any other shot that lands in its loosing tick waits so too (BDS's golem snowball, bds-snowgolem.txt).
#[derive(Component)]
pub(super) struct Struck;

/// A shot that hit or landed goes: the tick after where `later`, at once otherwise; whether it is
/// still there.
pub(super) fn gone(world: &mut World, shot: Entity, later: bool) -> Option<()> {
    if later {
        world.entity_mut(shot).insert(Struck);
        return Some(());
    }
    crate::mobs::remove(world, shot);
    None
}

/// A [`Struck`] shot goes, in the tick after it hit: what it leaves comes, and one two ticks old is
/// still heard `reappeared` (research/captures/bds-f14cthrow.txt, bds-p2shoot.txt).
pub(super) fn struck_goes(world: &mut World, shot: Entity, layer: Entity, id: u64, now: u64) {
    super::landing::leave(world, shot);
    if let (Some(born), Some(body)) = (world.get::<flight::Arrow>(shot).map(|arrow| arrow.born), world.get::<Body>(shot).copied()) {
        flight::vanish::reappear(world, shot, layer, id, now - born, &body);
    }
    crate::mobs::remove(world, shot);
}

/// Where the archer `by` stands: a player is thrown away from it, not from the shot
/// (research/captures/bds-pillswitch2.txt: pillagers 3 west and 1 north threw it 0.4 along (3, 1)).
fn archer_feet(world: &World, by: Entity) -> Option<[f32; 3]> {
    world.get::<Position>(by).map(|position| position.feet).or_else(|| world.get::<Body>(by).map(|body| body.at))
}

/// Whether `shot` is a loosed trident with Channeling.
pub(crate) fn channels(world: &World, shot: Entity) -> bool {
    world.get::<Loosed>(shot).and_then(|loosed| loosed.item.as_ref()).is_some_and(|item| enchant::level(item.tag.as_ref(), enchant::Enchant::Channeling) > 0)
}

/// The shot `arrow`, flying at `motion`, met `struck` at `at`.
pub(super) fn hit(world: &mut World, arrow: Entity, layer: Entity, struck: Entity, is_player: bool, at: [f32; 3], motion: [f32; 3]) -> After {
    let Some((archer, by, gives, shot)) = flight::shooter(world, arrow) else { return After::Gone };
    // Its hurt does nothing, so the shot is refused as by anything that takes nothing.
    if super::struck_back::strike(world, struck, by, motion) {
        if shot.kind == super::arrows::ARROW.kind {
            return After::Bounced;
        }
        if let Some(mut loosed) = world.get_mut::<Loosed>(arrow) {
            loosed.spent = true;
        }
        return After::Back;
    }
    let loosed = world.get::<Loosed>(arrow).cloned();
    let speed = motion.iter().map(|c| c * c).sum::<f32>().sqrt();
    let from: [f32; 3] = std::array::from_fn(|axis| at[axis] - motion[axis]);
    let cost = harm(world, layer, shot.harm, loosed.as_ref(), speed, struck, is_player);
    let push = loosed.as_ref().map_or([0.0; 3], |loosed| punched(loosed.punch, motion));
    let turns = if loosed.as_ref().is_none_or(|loosed| loosed.player.is_none()) { vitals::blocks_mob } else { vitals::blocks };
    if is_player && turns(world, struck, from, cost, Cause::Projectile, false) {
        if let Some(mut loosed) = world.get_mut::<Loosed>(arrow) {
            loosed.spent = true;
        }
        return After::Deflected;
    }
    let (landed, refused) = match is_player {
        // One proof against fire lost nothing to a fireball on BDS (bds-blaze.txt).
        true => {
            let landed = !(loosed.is_none() && shot.burns > 0 && vitals::is_fireproof(world, struck)) && vitals::shot(world, struck, cost, &archer, archer_feet(world, by).unwrap_or(from), shot.throws, push);
            (landed, !landed || vitals::resists_all(world, struck))
        }
        false => {
            let wound = harm::wound_by(world, struck, layer, cost, Cause::Projectile, loosed.as_ref().and_then(|loosed| loosed.player));
            (wound == Some(true), wound == Some(false) || world.get::<Health>(struck).is_some_and(Health::resists_all))
        }
    };
    if landed {
        afflict(world, struck, is_player, gives, loosed.as_ref().and_then(|loosed| loosed.tipped), from);
        burn(world, struck, is_player, shot.burns);
        if let Some(loosed) = loosed.as_ref().filter(|_| !is_player) {
            throw_mob(world, struck, layer, motion, push, loosed.player);
        }
        if !is_player {
            hunting::hurt_by(world, struck, by);
        }
    }
    if channels(world, arrow) {
        let feet = world.get::<crate::components::Position>(struck).map(|position| position.feet).or_else(|| world.get::<Body>(struck).map(|body| body.at));
        crate::lightning::channel(world, layer, feet.unwrap_or(at));
    }
    if shot.sticks {
        late_sounds::play_later(world, 1, layer, at.map(|c| c.floor() as i32), flight::sound(shot.heard[0], at, ""));
    }
    // Off one that took nothing (a resistance V player or villager, a guard) BDS's arrow bounces back (bds-n3knock.txt, bds-pillswitch2.txt).
    if refused && shot.kind == super::arrows::ARROW.kind {
        return After::Bounced;
    }
    let Some(mut loosed) = world.get_mut::<Loosed>(arrow) else { return After::Gone };
    if shot.kind != super::arrows::ARROW.kind {
        loosed.spent = true;
        return After::Back;
    }
    if loosed.pierces == 0 {
        return After::Gone;
    }
    loosed.pierces -= 1;
    loosed.pierced.push(struck);
    After::On
}

/// What the shot costs: a mob's arrow its speed twice over, rounded up, or its kind's own; a
/// player's as `sapling_data::shots` reckons, a trident more by Impaling on what is wet.
fn harm(world: &mut World, layer: Entity, own: Option<f32>, loosed: Option<&Loosed>, speed: f32, struck: Entity, is_player: bool) -> f32 {
    let Some(loosed) = loosed else { return own.unwrap_or_else(|| shots::arrow_harm(speed, 0, false, 0.0)) };
    if let Some(own) = own {
        let body = world.get::<Body>(struck).copied();
        let soaked = !is_player && body.is_some_and(|body| body.liquid() == Some(Liquid::Water));
        let stands = body.map(|body| (body.at, body.size[1])).or_else(|| world.get::<Position>(struck).map(|at| (at.feet, PLAYER_HEIGHT)));
        let wet = soaked || stands.is_some_and(|(feet, height)| harm::in_rain(world, layer, feet, height));
        return own + if wet { enchant::impaling(loosed.impaling) } else { 0.0 };
    }
    let roll = world.get_resource_or_insert_with(Dice::default).fraction();
    shots::arrow_harm(speed, loosed.power, loosed.critical, roll)
}

/// Punch's throw along the shot's way across.
fn punched(level: u8, motion: [f32; 3]) -> [f32; 3] {
    let across = motion[0].hypot(motion[2]).max(1e-4);
    let [along, up] = enchant::punch(level);
    if along == 0.0 { [0.0; 3] } else { [motion[0] / across * along, up, motion[2] / across * along] }
}

/// The shot's effect: its archer's, at its first level, or its tip's potion at the share an arrow gives.
fn afflict(world: &mut World, struck: Entity, is_player: bool, gives: Option<(&'static str, u32)>, tipped: Option<i16>, from: [f32; 3]) {
    match (is_player, gives, tipped) {
        (true, Some((effect, ticks)), _) => vitals::afflict(world, struck, effect, ticks),
        (false, Some((effect, ticks)), _) => afflictions::afflict(world, struck, effect, ticks),
        (true, None, Some(data)) => vitals::splashed(world, struck, data, shots::TIPPED_SHARE, from),
        (false, None, Some(data)) => afflictions::splashed(world, struck, data, shots::TIPPED_SHARE),
        _ => {}
    }
}

fn burn(world: &mut World, struck: Entity, is_player: bool, ticks: u16) {
    match is_player {
        _ if ticks == 0 => {}
        true => vitals::set_alight(world, struck, ticks),
        false => {
            if let Some(mut health) = world.get_mut::<Health>(struck) {
                health.ignite(ticks);
            }
        }
    }
}

/// The tick a mob was thrown in: it does not move again in that tick (creatures.rs).
#[derive(Component)]
pub(super) struct Thrown(u64);

/// Whether `mob` was thrown this tick, so its own move waits for the next one.
pub(super) fn thrown_now(world: &World, mob: Entity) -> bool {
    world.get::<Thrown>(mob).is_some_and(|thrown| thrown.0 == world.resource::<Tick>().0)
}

/// A shot or a hit throws the mob it met by `speed`, told to its viewers in the same tick: BDS sent a
/// struck cow's (0.4, 0.4) at once and its moved speed the tick after (bds-n3knock.txt), a slime
/// thrown by a golem that ticked first too (bds-j3slime.txt).
pub(super) fn throw_at_once(world: &mut World, mob: Entity, layer: Entity, speed: [f32; 3]) -> Option<()> {
    let mut body = world.get_mut::<Body>(mob)?;
    body.throw(speed);
    let at = body.at;
    let id = world.get::<NetId>(mob)?.0;
    let now = world.resource::<Tick>().0;
    world.entity_mut(mob).insert(Thrown(now));
    let told = SetEntityMotion { runtime_entity_id: id, velocity: Vec3f { x: speed[0], y: speed[1], z: speed[2] }, tick: 0 };
    entities::tell_viewers(world, mob, layer, at.map(|c| c.floor() as i32), vec![sapling_net::encode(&told)]);
    Some(())
}

/// A mob a player's shot hurt is thrown along the shot's way, and turns on the player.
fn throw_mob(world: &mut World, mob: Entity, layer: Entity, motion: [f32; 3], push: [f32; 3], player: Option<Entity>) -> Option<()> {
    let across = motion[0].hypot(motion[2]).max(1e-4);
    throw_at_once(world, mob, layer, [motion[0] / across * THROW + push[0], THROW + push[1], motion[2] / across * THROW + push[2]])?;
    let (id, body, kind) = (world.get::<NetId>(mob)?.0, *world.get::<Body>(mob)?, world.get::<entities::Kind>(mob)?.0.clone());
    hunting::provoke(world, mob, layer, id, &kind, &body, player?);
    super::strolling::startle(world, mob);
    Some(())
}
