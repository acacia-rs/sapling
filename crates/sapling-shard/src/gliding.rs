//! Elytra flight, as BDS ran it (research/captures/bds-glide.txt). The client flies itself; the
//! server starts a glide its input asks for when the player wears an elytra that still flies, is
//! off the ground and rides nothing, and ends it on the ground, in a vehicle, out of survival's
//! modes or when the elytra wears out, without waiting to be asked. Either edge is told to the
//! player and its viewers with the flag and the box's height. A second of gliding wears the
//! elytra by one; a rocket used while gliding boosts the glider for twice the rocket's life
//! (`MovementEffect`), as the client predicts from its use.
// TODO: water and levitation do not end a glide here; BDS's checks for them were not drilled.

use acacia_proto::packets::MovementEffect;
use acacia_proto::types::MovementEffectType;
use bevy_ecs::prelude::*;
use sapling_api::GameMode;

use crate::components::{InLayer, Mode, NetId, Outgoing, Position, Stage};
use crate::movement::{self, Motion};
use crate::{fireworks, inventory};

/// Ticks of gliding between two wears of the elytra.
const WEAR_EVERY: u64 = 20;

/// How long a player has been gliding, in ticks.
#[derive(Component, Default)]
struct Glide {
    ticks: u64,
}

/// Whether the player is gliding.
pub(crate) fn is_gliding(world: &World, player: Entity) -> bool {
    world.get::<Motion>(player).is_some_and(|motion| motion.pose.gliding)
}

/// The glider's velocity, as its client last stepped it; `None` for a player not gliding.
// TODO: read by the fly-into-wall damage, which combat (damage.rs) is to add.
#[allow(dead_code)]
pub(crate) fn velocity(world: &World, player: Entity) -> Option<[f32; 3]> {
    world.get::<Motion>(player).filter(|motion| motion.pose.gliding).map(|motion| motion.delta)
}

fn wears(world: &World, player: Entity) -> bool {
    world.get::<Mode>(player).is_some_and(|mode| matches!(mode.0, GameMode::Survival | GameMode::Adventure))
}

/// Whether the player could glide now: in the air, riding nothing, not a spectator, alive, with
/// an elytra that flies.
fn may_glide(world: &World, player: Entity) -> bool {
    let airborne = world.get::<Position>(player).is_some_and(|position| !position.on_ground);
    let free = world.get::<crate::riding::Riding>(player).is_none() && world.get::<Mode>(player).is_some_and(|mode| mode.0 != GameMode::Spectator);
    let alive = !crate::vitals::is_dead(world, player);
    airborne && free && alive && world.get::<inventory::Inventory>(player).is_some_and(inventory::elytra_flies)
}

/// Starts or ends a glide and tells the player and, by `movement::announce`, its viewers.
fn set(world: &mut World, player: Entity, gliding: bool) {
    let Some(id) = world.get::<NetId>(player).map(|id| id.0) else { return };
    let Some(mut motion) = world.get_mut::<Motion>(player) else { return };
    motion.pose.gliding = gliding;
    let shown = movement::pose_packet(id, motion.pose, motion.input_tick());
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.push(&shown);
    }
    match gliding {
        true => drop(world.entity_mut(player).insert(Glide::default())),
        false => drop(world.entity_mut(player).remove::<Glide>()),
    }
}

/// One tick of one player: what its inputs asked, what ends its glide, and the elytra's wear.
fn tick_one(world: &mut World, player: Entity) {
    let Some(asked) = world.get_mut::<Motion>(player).map(|mut motion| motion.glide_asked.take()) else { return };
    let gliding = is_gliding(world, player);
    match (gliding, asked) {
        (false, Some(true)) if may_glide(world, player) => return set(world, player, true),
        // A refused start is undone on the client, which shows it gliding already.
        (false, Some(true)) => return set(world, player, false),
        (true, Some(false)) => return set(world, player, false),
        (false, _) => return,
        (true, _) => {}
    }
    if !may_glide(world, player) {
        return set(world, player, false);
    }
    let ticks = world.get_mut::<Glide>(player).map(|mut glide| {
        glide.ticks += 1;
        glide.ticks
    });
    if ticks.is_some_and(|ticks| ticks % WEAR_EVERY == 0) && wears(world, player) && !inventory::wear_elytra(world, player) {
        set(world, player, false);
    }
}

/// Answers every player's glide for this tick.
pub(crate) fn tick(world: &mut World) {
    let players: Vec<Entity> = world.query_filtered::<(Entity, &Stage), With<Motion>>().iter(world).filter(|(_, stage)| **stage == Stage::InWorld).map(|(player, _)| player).collect();
    for player in players {
        tick_one(world, player);
    }
}

/// A use in the air of the rocket in a glider's hand: the rocket rides along and the glider is
/// boosted for twice its life, stamped with the input's tick; in survival one rocket is spent and
/// the windows sent again, the slot selected, as BDS did. False when the player is not gliding
/// or holds no rocket.
pub(crate) fn boost(world: &mut World, player: Entity) -> bool {
    let held = world.get::<inventory::Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref()).map(|held| held.stack.clone());
    let Some(stack) = held.filter(|stack| stack.name == fireworks::ROCKET && is_gliding(world, player)) else { return false };
    let (Some(layer), Some(id), Some(tick)) = (world.get::<InLayer>(player).map(|layer| layer.0), world.get::<NetId>(player).map(|id| id.0), world.get::<Motion>(player).map(Motion::input_tick)) else { return false };
    let Some(lasts) = fireworks::boost(world, layer, player, &stack) else { return true };
    let effect = MovementEffect { runtime_id: id, effect_type: MovementEffectType::GLIDEBOOST, effect_duration: (lasts * 2) as u32, tick };
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.push(&effect);
    }
    if world.get::<Mode>(player).is_some_and(|mode| mode.0 != GameMode::Creative) {
        if let Some(mut inventory) = world.get_mut::<inventory::Inventory>(player) {
            inventory.take_one_in_hand();
        }
        inventory::show_windows(world, player, true);
    }
    true
}
