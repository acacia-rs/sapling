//! A bat hangs from the block over its head (the Java game's `Bat::customServerAiStep`), as BDS's
//! did (research/captures/bds-g14bat.txt, bds-g14fly.txt): one born under a ceiling hangs from the
//! start, its `AddEntity` showing `RESTING`; one flying under one takes hold at 1 in 100 a tick
//! (BDS's took 5 s). It hangs still, its head against the ceiling, and lets go, heard `takeoff` from its
//! feet, the tick the block over it goes or a player not spectating comes within 4, creative too
//! (bds-x2bat.txt: woken 3.2 off, kept 4.1 off; one born that near is never shown `RESTING`).

use acacia_proto::types::MetadataFlags1 as Flags;
use bevy_ecs::prelude::*;
use sapling_api::GameMode;

use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Layer, Mode, NetId, Position, in_world};
use crate::entities;

const KIND: &str = "minecraft:bat";
const TAKES_HOLD: f32 = 0.01;
const WAKES: f32 = 4.0;

#[derive(Component)]
pub(super) struct Hanging;

/// The flags a bat of `kind` born as `body` in `layer` is shown with: `RESTING` under a ceiling with no player near.
pub(super) fn born_flags(world: &World, layer: Entity, kind: &str, body: &Body) -> Flags {
    let under = world.get::<Layer>(layer).is_some_and(|level| ceiling(&level.world, body));
    if kind == KIND && under && !woken(world, layer, body) { Flags::RESTING } else { Flags(0) }
}

/// The flags the mob shows as it is announced again: `RESTING` while it hangs.
pub(super) fn hanging_flags(world: &World, entity: Entity) -> Flags {
    if world.get::<Hanging>(entity).is_some() { Flags::RESTING } else { Flags(0) }
}

/// A bat born under a ceiling holds on, to let go at once if a player is near.
pub(super) fn arm(world: &mut World, entity: Entity, layer: Entity, kind: &str) {
    let (Some(body), Some(level)) = (world.get::<Body>(entity).copied(), world.get::<Layer>(layer).map(|layer| &layer.world)) else { return };
    if kind == KIND && ceiling(level, &body) {
        world.entity_mut(entity).insert(Hanging);
        if born_flags(world, layer, kind, &body) != Flags(0) {
            entities::mob_flags(world, entity, kind, Flags::RESTING, true);
        }
        hold(world, entity, &body);
    }
}

fn ceiling(level: &sapling_world::World, body: &Body) -> bool {
    let [x, y, z] = body.at.map(|c| c.floor() as i32);
    level.in_height(y + 1) && super::harm::is_full_block(level, [x, y + 1, z])
}

/// Whether a player that is not spectating is within [`WAKES`] of the bat.
fn woken(world: &World, layer: Entity, body: &Body) -> bool {
    in_world(world).into_iter().any(|player| {
        let here = world.get::<InLayer>(player).is_some_and(|in_layer| in_layer.0 == layer) && world.get::<Mode>(player).is_some_and(|mode| mode.0 != GameMode::Spectator);
        here && world.get::<Position>(player).is_some_and(|position| super::hunting::distance(position.feet, body.at) < WAKES)
    })
}

/// Before its move: a hanging bat holds on, or lets go as its ceiling goes or a player comes near; a
/// flying one under a ceiling may take hold. False for a bat flying on, or another kind.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, kind: &str) -> bool {
    let (true, Some(body), Some(level)) = (kind == KIND, world.get::<Body>(entity).copied(), world.get::<Layer>(layer).map(|layer| &layer.world)) else { return false };
    let under = ceiling(level, &body);
    let hanging = world.get::<Hanging>(entity).is_some();
    let woken = woken(world, layer, &body);
    let takes_hold = !hanging && under && !woken && world.get_resource_or_insert_with(Dice::default).fraction() < TAKES_HOLD;
    if takes_hold {
        world.entity_mut(entity).insert(Hanging);
        super::husbandry::show_flag(world, entity, Flags::RESTING, true);
    } else if hanging && (!under || woken) {
        world.entity_mut(entity).remove::<Hanging>();
        super::husbandry::show_flag(world, entity, Flags::RESTING, false);
        take_off(world, entity, layer, &body);
    }
    let holds = world.get::<Hanging>(entity).is_some();
    if holds {
        hold(world, entity, &body);
    }
    holds
}

fn take_off(world: &mut World, entity: Entity, layer: Entity, body: &Body) {
    let Some(id) = world.get::<NetId>(entity).map(|id| id.0) else { return };
    let cell = body.at.map(|c| c.floor() as i32);
    entities::tell_viewers(world, entity, layer, cell, vec![super::hunting::sound("takeoff", id, KIND, body, 0.0)]);
}

fn hold(world: &mut World, entity: Entity, body: &Body) {
    let top = body.at[1].floor() + 1.0 - body.size[1];
    if let Some(mut body) = world.get_mut::<Body>(entity) {
        body.at[1] = top;
        body.fling([0.0; 3]);
    }
}
