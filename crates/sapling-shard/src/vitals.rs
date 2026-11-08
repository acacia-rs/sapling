//! A player's health: what a fall, fire, lava, water over its head and a block around it take
//! from it each tick, by the rules mobs are hurt by (creatures/health.rs), told as BDS tells a
//! player's (docs/research/player-damage.md). A hit: harm.rs; dying and rising: death.rs; mending
//! and starving: hunger.rs; effects: effects.rs.
// Crawling under a block: movement/crawl.rs.

pub(crate) mod boost;
mod chorus;
mod death;
pub(crate) mod eating;
pub(crate) mod effects;
pub(crate) mod frost;
pub(crate) use eating::bite;
pub(crate) use effects::{afflict, give as give_effect, give_ambient, hits_for, kept as kept_effects, restored as restored_effects};
pub(crate) use struck::{bitten, blasted, boomed, crushed, electrocuted, fanged, fireworked, is_fireproof, knocked, pearled, Punch, punched, pushed, rammed, resists_all, set_alight, shot, thorned, winded};
mod harm;
use harm::{ask, dress, flash, land, settle};
pub(crate) use harm::{flash_guarded, hurt};
mod hazards;
mod hunger;
mod kept;
mod last_death;
pub(crate) use last_death::{LastDeath, greeting as last_death_greeting};
mod omens;
mod rise_spot;
mod splashed;
mod stand;
mod shield;
mod soles;
mod struck;
pub(crate) mod told_all;
mod told_health;
pub(crate) use told_health::{held as health_held, tell_held as tell_held_health};
mod totem;
pub(crate) use shield::{Guard, blocks, blocks_mob, shown_bits, thrown_back, wields_axe};
pub(crate) use stand::{break_fall, breathes_water, fallen, falls_freely};
#[cfg(test)]
pub(crate) use splashed::shared_strength;
pub(crate) use splashed::{clouded, health_left, is_under, splashed};

use acacia_proto::Packet;
use bevy_ecs::prelude::*;
use sapling_api::GameMode;

use crate::components::{InLayer, Mode, NetId, Outgoing, Position, Tick, in_world};
use crate::creatures::Health;
use crate::visibility::ToViewers;

const MAX: f32 = 20.0;
/// A player's box: across, and up.
pub(crate) const SIZE: [f32; 2] = [0.6, 1.8];
const FIERY: &[&str] = &["minecraft:lava", "minecraft:flowing_lava", "minecraft:fire", "minecraft:soul_fire"];
const LAVA: &[&str] = &["minecraft:lava", "minecraft:flowing_lava"];
const WATER: &[&str] = &["minecraft:water", "minecraft:flowing_water"];
/// Ticks of breath, and what a tick in the open gives back.
const AIR: i16 = 300;
const BREATH: i16 = 4;
/// Out of breath a player holds on this many ticks, is hurt, and holds on again.
const GASP: i16 = 20;
const DROWN_HARM: f32 = 2.0;
/// Ticks a player is in a block before the block hurts it: one more than a mob.
const WALLED: u8 = 7;
/// Exhaustion of a jump, of a jump in a sprint, and of a block sprinted on the ground
/// (research/captures/bds-tire.txt); walking costs none.
const JUMP_TIRES: f32 = 0.05;
const LEAP_TIRES: f32 = 0.2;
const SPRINT_TIRES: f32 = 0.1;
/// Exhaustion of a block broken with what harvests it, and of a hit on a mob in reach
/// (research/captures/bds-toil.txt).
pub(crate) const BREAK_TIRES: f32 = 0.005;
pub(crate) const HIT_TIRES: f32 = 0.1;
/// No tick's run is longer: more is a teleport or a throw.
const LONGEST_STEP: f32 = 1.0;

#[derive(Component, Clone, Copy)]
pub(crate) struct Vitals {
    health: Health,
    air: i16,
    hunger: hunger::Hunger,
    meal: Option<eating::Meal>,
    /// What the client and the viewers know of its health, and of the cushion on it.
    told: f32,
    padded: f32,
    /// Where its feet were a tick ago.
    was: Option<[f32; 3]>,
    /// The tick it died in, until it rises.
    died: Option<u64>,
    /// Experience its body leaves as it goes.
    leaves: Option<i32>,
    /// What storage last kept of it (kept.rs).
    saved: kept::Kept,
}

impl Default for Vitals {
    fn default() -> Self {
        let hunger = hunger::Hunger::default();
        let [fed, saturation, exhaustion] = hunger.kept();
        let saved = kept::Kept { vitals: [MAX, fed, saturation, exhaustion], air: AIR, fire: 0 };
        Self { health: Health::new(MAX, None, 0), air: AIR, hunger, meal: None, told: MAX, padded: 0.0, was: None, died: None, leaves: None, saved }
    }
}

impl Vitals {
    /// Eating or drinking, which slows the player's walk.
    pub(crate) fn is_eating(&self) -> bool {
        self.meal.is_some()
    }
}

pub(crate) fn is_mortal(world: &World, player: Entity) -> bool {
    !is_dead(world, player) && world.get::<Mode>(player).is_some_and(|mode| matches!(mode.0, GameMode::Survival | GameMode::Adventure))
}

pub(crate) fn is_creative(world: &World, player: Entity) -> bool {
    !is_dead(world, player) && world.get::<Mode>(player).is_some_and(|mode| mode.0 == GameMode::Creative)
}

/// Its health whole again, as BDS's respawn after the End's exit portal makes it; the caller
/// tells it (portals/exit.rs).
pub(crate) fn mend_fully(world: &mut World, player: Entity) {
    if let Some(mut vitals) = world.get_mut::<Vitals>(player) {
        vitals.health.mend(MAX);
        vitals.told = MAX;
    }
}

/// Died and not yet risen.
pub(crate) fn is_dead(world: &World, player: Entity) -> bool {
    world.get::<Vitals>(player).is_none_or(|vitals| vitals.died.is_some())
}

/// `event` heard the next tick from the cell of the player's eyes, as BDS's hurt player was
/// (research/captures/bds-b1sculk.txt).
fn heard_from_eyes(world: &mut World, player: Entity, event: crate::blocks::GameEvent) {
    let (Some(position), Some(layer)) = (world.get::<Position>(player).copied(), world.get::<InLayer>(player).map(|layer| layer.0)) else { return };
    let [x, y, z] = position.feet;
    let eyes = [x, y + sapling_data::EYE_HEIGHT, z].map(|c| c.floor() as i32);
    crate::blocks::vibrate_next_tick(world, layer, eyes, event);
}

/// To the player and to those who see it.
pub(crate) fn tell<T: Packet>(world: &mut World, player: Entity, packet: &T) {
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.push(packet);
    }
    world.resource_mut::<ToViewers>().push(player, packet);
}

fn layer_of(world: &World, player: Entity) -> Option<Entity> {
    world.get::<InLayer>(player).map(|in_layer| in_layer.0)
}

/// Whether work and hunger effects tire the player: `sapling:hunger` is on in its world.
fn tires(world: &World, player: Entity) -> bool {
    layer_of(world, player).is_some_and(|layer| crate::rules::on(world, layer, sapling_data::HUNGER))
}

/// Changes a survival player's hunger, told at once where what is shown of it changed.
fn feed(world: &mut World, player: Entity, change: impl FnOnce(&mut hunger::Hunger)) {
    let (Some(id), true) = (world.get::<NetId>(player).map(|id| id.0), is_mortal(world, player)) else { return };
    let Some(mut vitals) = world.get_mut::<Vitals>(player) else { return };
    let before = vitals.hunger.shown();
    change(&mut vitals.hunger);
    let (hunger, tick) = (vitals.hunger, world.resource::<Tick>().0);
    if hunger.shown() != before {
        tell(world, player, &hunger.told(id, tick));
    }
}

/// Work tires a survival player by `by`.
pub(crate) fn tire(world: &mut World, player: Entity, by: f32) {
    if tires(world, player) {
        feed(world, player, |hunger| hunger.tire(by));
    }
}

/// Lets the dead rise who asked to, hurts the living by where they stand, and lets the bodies go.
pub(crate) fn tick(world: &mut World) {
    omens::tick(world);
    death::rise(world);
    eating::tick(world);
    effects::tick(world);
    shield::tick(world);
    let players: Vec<(Entity, Entity)> = in_world(world).into_iter().filter_map(|player| Some((player, world.get::<InLayer>(player)?.0))).collect();
    for (player, layer) in players {        if is_mortal(world, player) {
            stand::stand(world, player, layer);
        } else {
            stand::stand_unhurt(world, player, layer);
            if let Some(died) = world.get::<Vitals>(player).and_then(|vitals| vitals.died) {
                frost::thaw_dead(world, player, world.resource::<Tick>().0 - died);
            }
            death::leave(world, player, layer);
        }
    }
}
