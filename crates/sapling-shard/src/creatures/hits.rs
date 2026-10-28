//! What a hurting shot does to the player or mob it meets (flight.rs): it costs what
//! `sapling_data::shots` reckons of its speed and a player's Power and draw, throws as a hit does,
//! further by Punch, gives its archer's effect or its tip's potion, and sets alight what Flame or a
//! fireball burns. A mob a player's shot kills is the player's. One that pierces flies on; a
//! trident bounces off and hurts nothing more.

use bevy_ecs::prelude::*;
use sapling_data::{enchant, shots};

use super::flight::{self, Loosed};
use super::health::{Cause, Health};
use super::{afflictions, harm, hunting};
use crate::blocks::Dice;
use crate::body::{Body, Liquid};
use crate::components::NetId;
use crate::{entities, late_sounds, vitals};

/// A player's shot throws the mob it hits as a hit does: this fast away and up.
// TODO: the melee throw (attacks.rs); BDS's for a projectile was not measured.
const THROW: f32 = 0.4;

/// What becomes of a shot that hit.
pub(super) enum After {
    Gone,
    /// It flies on through (Piercing).
    On,
    /// It bounces back off (a trident).
    Back,
    /// A shield turned it away, a tenth as fast (research/captures/bds-shield.txt).
    Deflected,
}

/// The shot `arrow`, flying at `motion`, met `struck` at `at`.
pub(super) fn hit(world: &mut World, arrow: Entity, layer: Entity, struck: Entity, is_player: bool, at: [f32; 3], motion: [f32; 3]) -> After {
    let Some((archer, _, gives, shot)) = flight::shooter(world, arrow) else { return After::Gone };
    let loosed = world.get::<Loosed>(arrow).cloned();
    let speed = motion.iter().map(|c| c * c).sum::<f32>().sqrt();
    let from: [f32; 3] = std::array::from_fn(|axis| at[axis] - motion[axis]);
    let cost = harm(world, layer, shot.harm, loosed.as_ref(), speed, struck, is_player);
    let push = loosed.as_ref().map_or([0.0; 3], |loosed| punched(loosed.punch, motion));
    if is_player && vitals::blocks(world, struck, from, cost, Cause::Projectile, false) {
        if let Some(mut loosed) = world.get_mut::<Loosed>(arrow) {
            loosed.spent = true;
        }
        return After::Deflected;
    }
    let landed = match is_player {
        // One proof against fire lost nothing to a fireball on BDS (bds-blaze.txt).
        true => !(loosed.is_none() && shot.burns > 0 && vitals::is_fireproof(world, struck)) && vitals::shot(world, struck, cost, &archer, from, shot.throws, push),
        false => harm::wound_by(world, struck, layer, cost, Cause::Projectile, loosed.as_ref().and_then(|loosed| loosed.player)) == Some(true),
    };
    if landed {
        afflict(world, struck, is_player, gives, loosed.as_ref().and_then(|loosed| loosed.tipped), from);
        burn(world, struck, is_player, shot.burns);
        if let Some(loosed) = loosed.as_ref().filter(|_| !is_player) {
            throw_mob(world, struck, layer, motion, push, loosed.player);
        }
    }
    if loosed.as_ref().and_then(|loosed| loosed.item.as_ref()).is_some_and(|item| enchant::level(item.tag.as_ref(), enchant::Enchant::Channeling) > 0) {
        let feet = world.get::<crate::components::Position>(struck).map(|position| position.feet).or_else(|| world.get::<Body>(struck).map(|body| body.at));
        crate::lightning::channel(world, layer, feet.unwrap_or(at));
    }
    if shot.sticks {
        late_sounds::play_later(world, 1, layer, at.map(|c| c.floor() as i32), flight::sound(shot.heard[0], at, ""));
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
        let wet = !is_player && world.get::<Body>(struck).is_some_and(|body| body.liquid() == Some(Liquid::Water)) || crate::weather::of(world, layer) != sapling_api::Weather::Clear;
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

/// A mob a player's shot hurt is thrown along the shot's way, and turns on the player.
fn throw_mob(world: &mut World, mob: Entity, layer: Entity, motion: [f32; 3], push: [f32; 3], player: Option<Entity>) -> Option<()> {
    let across = motion[0].hypot(motion[2]).max(1e-4);
    let mut body = world.get_mut::<Body>(mob)?;
    body.throw([motion[0] / across * THROW + push[0], THROW + push[1], motion[2] / across * THROW + push[2]]);
    let (id, body, kind) = (world.get::<NetId>(mob)?.0, *world.get::<Body>(mob)?, world.get::<entities::Kind>(mob)?.0.clone());
    hunting::provoke(world, mob, layer, id, &kind, &body, player?);
    super::strolling::startle(world, mob);
    Some(())
}
