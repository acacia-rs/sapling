//! A player's health: what a fall, fire, lava, water over its head and a block around it take
//! from it each tick, by the rules mobs are hurt by (creatures/health.rs), told as BDS tells a
//! player's (docs/research/player-damage.md). Dying and rising: death.rs; mending and
//! starving: hunger.rs; effects: effects.rs.
// TODO: a block over the head alone does not
// make a player crawl; the breathing flag is not told; plugins are not asked.

mod death;
pub(crate) mod eating;
mod effects;
pub(crate) use effects::{afflict, hits_for, kept as kept_effects, restored as restored_effects};
pub(crate) use struck::{bitten, blasted, shot};
mod hunger;
mod kept;
mod struck;

use acacia_proto::Packet;
use acacia_proto::packets::{EntityEvent, EntityEventEventId};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{DamageCause, GameMode, HealthBackend, PlayerId};
use sapling_data::EYE_HEIGHT;

use crate::body::Body;
use crate::components::{InLayer, Layer, Mode, NetId, Outgoing, Position, Tick, in_world};
use crate::creatures::{Cause, Health, holds, is_full_block, touches};
use crate::handlers::Ctx;
use crate::movement::{self, Motion};
use crate::visibility::ToViewers;

const MAX: f32 = 20.0;
/// A player's box: across, and up.
const SIZE: [f32; 2] = [0.6, 1.8];
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
/// Exhaustion of a block broken with what harvests it, and of a hit that lands
/// (research/captures/bds-toil.txt).
pub(crate) const BREAK_TIRES: f32 = 0.005;
pub(crate) const HIT_TIRES: f32 = 0.1;
/// No tick's run is longer: more is a teleport or a throw.
const LONGEST_STEP: f32 = 1.0;
const RESISTS: f32 = 0.2;
/// What a helmet lets through of a hit on the head (research/captures/bds-worn.txt).
const HELMETED: f32 = 0.75;
/// Health an empty hunger leaves a player (hunger.rs).
const STARVED: f32 = 10.0;

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
    saved: [f32; 4],
}

impl Default for Vitals {
    fn default() -> Self {
        let hunger = hunger::Hunger::default();
        let [fed, saturation, exhaustion] = hunger.kept();
        Self { health: Health::new(MAX, None, 0), air: AIR, hunger, meal: None, told: MAX, padded: 0.0, was: None, died: None, leaves: None, saved: [MAX, fed, saturation, exhaustion] }
    }
}

pub(crate) fn is_mortal(world: &World, player: Entity) -> bool {
    let is_alive = world.get::<Vitals>(player).is_some_and(|vitals| vitals.died.is_none());
    is_alive && world.get::<Mode>(player).is_some_and(|mode| matches!(mode.0, GameMode::Survival | GameMode::Adventure))
}

/// To the player and to those who see it.
fn tell<T: Packet>(world: &mut World, player: Entity, packet: &T) {
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.push(packet);
    }
    world.resource_mut::<ToViewers>().push(player, packet);
}

/// The flash of a hit of `cause` that landed; a fall's names the player's eyes.
fn flash(world: &mut World, player: Entity, cause: Cause) {
    let (Some(id), Some(position)) = (world.get::<NetId>(player).map(|id| id.0), world.get::<Position>(player).copied()) else { return };
    let [x, y, z] = position.feet;
    let fire_at_position = (cause == Cause::Fall).then_some(Vec3f { x, y: y + EYE_HEIGHT, z });
    tell(world, player, &EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::HurtAnimation, data: cause.id(), fire_at_position });
}

/// Tells the health and the burning where they changed, and lets a player with no health die of `cause`.
fn settle(world: &mut World, player: Entity, cause: Cause) {
    let Some(mut vitals) = world.get_mut::<Vitals>(player) else { return };
    let (left, burning, most) = (vitals.health.left(), vitals.health.is_burning(), vitals.health.max());
    let changed = std::mem::replace(&mut vitals.told, left) != left;
    let cushion = vitals.health.cushion();
    let padded = std::mem::replace(&mut vitals.padded, cushion) != cushion;
    let Some(id) = world.get::<NetId>(player).map(|id| id.0) else { return };
    let tick = world.resource::<Tick>().0;
    if padded {
        tell(world, player, &effects::cushion(id, cushion, tick));
    }
    if changed {
        tell(world, player, &sapling_data::mob_health_update(id, left, most));
    }
    if let Some(mut motion) = world.get_mut::<Motion>(player).filter(|motion| motion.pose.burning != burning) {
        motion.pose.burning = burning;
        let shown = movement::pose_packet(id, motion.pose, tick);
        // Viewers are told by `movement::announce`.
        world.get_mut::<Outgoing>(player).expect("players have an outbox").push(&shown);
    }
    if left <= 0.0 {
        death::die(world, player, cause);
    }
}

/// Has the player's health know the armour it wears now and what its effects ward off: a
/// level of resistance takes a fifth off every hit (research/captures/bds-more.txt).
fn dress(world: &mut World, player: Entity) {
    let armour = world.get::<crate::inventory::Inventory>(player).map_or([0.0; 2], |inventory| inventory.armour());
    let resisted = effects::level(world, player, sapling_api::Effect::Resistance).map_or(0.0, |level| RESISTS * (f32::from(level) + 1.0));
    let fireproof = effects::level(world, player, sapling_api::Effect::FireResistance).is_some();
    // A level of jump boost forgives a block of a fall, slow falling all of it (bds-fx.txt).
    let floats = effects::level(world, player, sapling_api::Effect::SlowFalling).is_some();
    let springs = if floats { f32::INFINITY } else { effects::level(world, player, sapling_api::Effect::JumpBoost).map_or(0.0, |level| f32::from(level) + 1.0) };
    if let Some(mut vitals) = world.get_mut::<Vitals>(player) {
        vitals.health.wear(armour);
        vitals.health.ward((1.0 - resisted).max(0.0), fireproof);
        vitals.health.spring(springs);
    }
}

/// Takes `amount` from a player's health as a hit of `cause`, less what its armour stops of a
/// cause it stops; false where it took nothing.
fn hurt(world: &mut World, player: Entity, amount: f32, cause: Cause) -> bool {
    dress(world, player);
    let fiery = matches!(cause, Cause::Fire | Cause::FireTick | Cause::Lava | Cause::Magma);
    if fiery && world.get::<Vitals>(player).is_some_and(|vitals| vitals.health.is_fireproof()) {
        return false;
    }
    let helmeted = cause.is_on_the_head() && world.get::<crate::inventory::Inventory>(player).is_some_and(|inventory| inventory.has_helmet());
    let whole = amount;
    let amount = if helmeted { amount * HELMETED } else { amount };
    let lands = |mut vitals: Mut<Vitals>| {
        let amount = if cause.is_stopped_by_armour() { vitals.health.shielded(amount) } else { amount };
        vitals.health.hurt(amount)
    };
    if !is_mortal(world, player) || !world.get_mut::<Vitals>(player).is_some_and(lands) {
        return false;
    }
    if cause.is_stopped_by_armour() {
        crate::inventory::wear_armour(world, player, whole, cause.is_on_the_head());
    }
    if world.get::<Vitals>(player).is_some_and(|vitals| vitals.health.left() > 0.0) {
        flash(world, player, cause);
    }
    settle(world, player, cause);
    true
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
    feed(world, player, |hunger| hunger.tire(by));
}

/// A tick of what the place a living player stands in does to it.
fn stand(world: &mut World, player: Entity, layer: Entity) {
    dress(world, player);
    dress(world, player);
    let gilled = effects::level(world, player, sapling_api::Effect::WaterBreathing).is_some();
    let (Some(position), Some(mut vitals)) = (world.get::<Position>(player).copied(), world.get::<Vitals>(player).copied()) else { return };
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let body = Body::new(position.feet, [0.0; 3], SIZE);
    let (fire, lava, water) = (touches(level, &body, FIERY), touches(level, &body, LAVA), touches(level, &body, WATER));
    let [low, eyes] = [0.0, EYE_HEIGHT].map(|up| [position.feet[0], position.feet[1] + up, position.feet[2]].map(|c| c.floor() as i32));
    // BDS lays a player with only its head in a block down, unhurt.
    let (under_water, in_wall) = (holds(level, eyes, WATER) && !gilled, is_full_block(level, eyes) && is_full_block(level, low));
    // A teleport is no fall: until the client follows it, the height starts anew.
    let (carried, sprinting, jumps) = world.get_mut::<Motion>(player).map_or((false, false, 0), |mut motion| (motion.awaits_teleport(), motion.pose.sprinting, std::mem::take(&mut motion.jumps)));
    let from = vitals.was.replace(position.feet).unwrap_or(position.feet);
    let run = (position.feet[0] - from[0]).hypot(position.feet[2] - from[2]);
    let ran = if sprinting && position.on_ground && !carried && run <= LONGEST_STEP { run } else { 0.0 };
    let fed = vitals.hunger;
    vitals.hunger.tire(f32::from(jumps) * if sprinting { LEAP_TIRES } else { JUMP_TIRES } + ran * SPRINT_TIRES);
    let (health, breath) = (&mut vitals.health, vitals.air);
    let mut hits = Vec::new();
    health.begin();
    // BDS does not count the move a player lands in: 12 blocks cost it 8 (bds-hurt.txt).
    let fallen_to = if position.on_ground { position.feet[1].max(from[1]) } else { position.feet[1] };
    hits.extend(health.fall(from, fallen_to, position.on_ground, water || lava || carried).map(|_| Cause::Fall));
    let lit = health.is_burning();
    hits.extend(health.burn(fire, water).map(|cause| if lit { Cause::FireTick } else { cause }));
    hits.extend((lava && health.scald()).then_some(Cause::Lava));
    hits.extend(health.smother(in_wall, WALLED).then_some(Cause::Suffocation));
    vitals.air = if under_water || in_wall { breath - 1 } else { (breath + BREATH).min(AIR) };
    if vitals.air <= -GASP {
        vitals.air = 0;
        hits.extend((under_water && vitals.health.hurt(DROWN_HARM)).then_some(Cause::Drowning));
    }
    let left = vitals.health.left();
    match vitals.hunger.tick(left > 0.0 && left < vitals.health.max()) {
        Some(hunger::Turn::Mend) => vitals.health.mend(1.0),
        Some(hunger::Turn::Starve) => hits.extend((left > STARVED && vitals.health.hurt(1.0)).then_some(Cause::Starving)),
        None => {}
    }
    let is_alive = vitals.health.left() > 0.0;
    *world.get_mut::<Vitals>(player).expect("read above") = vitals;
    if let Some(id) = world.get::<NetId>(player).map(|id| id.0).filter(|_| vitals.air != breath) {
        tell(world, player, &sapling_data::mob_air(id, vitals.air));
    }
    if let Some(id) = world.get::<NetId>(player).map(|id| id.0).filter(|_| vitals.hunger.shown() != fed.shown()) {
        let tick = world.resource::<Tick>().0;
        tell(world, player, &vitals.hunger.told(id, tick));
    }
    for cause in hits.iter().filter(|_| is_alive) {
        flash(world, player, *cause);
    }
    settle(world, player, hits.last().copied().unwrap_or(Cause::Fire));
}

/// Lets the dead rise who asked to, hurts the living by where they stand, and lets the bodies go.
pub(crate) fn tick(world: &mut World) {
    death::rise(world);
    eating::tick(world);
    effects::tick(world);
    let players: Vec<(Entity, Entity)> = in_world(world).into_iter().filter_map(|player| Some((player, world.get::<InLayer>(player)?.0))).collect();
    for (player, layer) in players {
        if is_mortal(world, player) {
            stand(world, player, layer);
        } else {
            if let Some(mut vitals) = world.get_mut::<Vitals>(player) {
                vitals.was = None;
            }
            death::leave(world, player, layer);
        }
    }
}

impl HealthBackend for Ctx<'_> {
    fn health(&self, player: PlayerId) -> Option<f32> {
        self.world.get::<Vitals>(self.entity(player)?).map(|vitals| vitals.health.left())
    }

    fn hurt_player(&mut self, player: PlayerId, amount: f32, cause: DamageCause) -> bool {
        self.entity(player).is_some_and(|entity| hurt(self.world, entity, amount, cause))
    }

    fn kill_player(&mut self, player: PlayerId) -> bool {
        let Some(entity) = self.entity(player).filter(|entity| is_mortal(self.world, *entity)) else { return false };
        if let Some(mut vitals) = self.world.get_mut::<Vitals>(entity) {
            vitals.health.end();
        }
        settle(self.world, entity, Cause::Generic);
        true
    }
}
