//! A mob hurt by a command or a plugin, maybe as from a player or a mob. One from a source in the
//! world is that source's hit: BDS's cow was thrown from the player by `entity_attack`, `magic` and
//! `fall` alike (none without a source), and its wolf, zombie pigmen and iron golem took the player
//! for their prey, its skeleton the zombie named (research/captures/bds-h2dmgm.txt).

use bevy_ecs::prelude::*;
use sapling_api::{Attacker, DamageCause};

use super::health::Health;
use crate::body::Body;
use crate::components::{InLayer, NetId, Position};
use crate::entities;

/// Hurts the living `mob` by `amount` of `cause`, dealt by `by` (`player` where that is a player):
/// whether it took any.
pub(super) fn wound(world: &mut World, mob: Entity, amount: f32, cause: DamageCause, by: Option<&Attacker>, player: Option<Entity>) -> bool {
    if world.get::<super::Dying>(mob).is_some() {
        return false;
    }
    if let Some(took) = crate::vehicles::damaged(world, mob, amount, cause) {
        return took;
    }
    let (Some(layer), Some(health)) = (world.get::<InLayer>(mob).map(|layer| layer.0), world.get::<Health>(mob).copied()) else { return false };
    let before = health.left();
    // BDS's `/damage` of 8 cost a zombie in a diamond chestplate 5.824 (research/captures/bds-boosts.txt).
    let amount = if cause.is_stopped_by_armour() { health.shielded(amount) } else { amount };
    if let Some(Attacker::Mob { kind }) = by
        && super::villagers::infect(world, mob, layer, kind, amount)
    {
        return true;
    }
    super::harm::wound_by(world, mob, layer, amount, cause, player);
    world.get::<super::Dying>(mob).is_some() || world.get::<Health>(mob).is_some_and(|health| health.left() < before)
}

/// [`wound`]s the mob as from `source`, a player or another mob: one that took it and lives is
/// thrown away from the source and answers it as a hit.
pub(super) fn hurt(world: &mut World, mob: Entity, amount: f32, cause: DamageCause, source: Entity) -> bool {
    let source_kind = world.get::<entities::Kind>(source).map(|kind| kind.0.clone());
    let from = match source_kind {
        Some(_) => world.get::<Body>(source).map(|body| body.at),
        None => world.get::<Position>(source).map(|position| position.feet),
    };
    let player = source_kind.is_none().then_some(source);
    let by = match &source_kind {
        Some(kind) => Some(Attacker::Mob { kind: kind.clone() }),
        None => crate::components::in_world_identity(world, source).map(Attacker::Player),
    };
    let took = wound(world, mob, amount, cause, by.as_ref(), player);
    if !took || world.get::<super::Dying>(mob).is_some() || world.get::<crate::vehicles::Vehicle>(mob).is_some() {
        return took;
    }
    let (Some(from), Some(layer), Some(id), Some(kind)) = (from, world.get::<InLayer>(mob).map(|layer| layer.0), world.get::<NetId>(mob).map(|id| id.0), world.get::<entities::Kind>(mob).map(|kind| kind.0.clone())) else { return took };
    super::blows::throw_from(world, mob, from, 0, false);
    let Some(body) = world.get::<Body>(mob).copied() else { return took };
    match player {
        Some(player) => super::attacks::answer(world, mob, layer, id, &kind, &body, player),
        None => {
            super::strolling::startle(world, mob);
            super::hunting::hurt_by(world, mob, source);
            super::teleports::startle(world, mob);
        }
    }
    took
}
