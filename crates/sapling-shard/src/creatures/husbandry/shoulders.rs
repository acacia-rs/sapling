//! A tamed parrot on a player's shoulder (its pack's `find_mount`; the player's `rideable` seats,
//! riding/seats.rs), as BDS did it (research/captures/bds-as3parrot.txt): 100 ticks after it could
//! first, a tamed parrot that neither sits nor rides flies to the nearest player within 16 with a
//! shoulder free, owner or not, and gets on within 2. Parrots get off, the newest first, once the
//! player has fallen 2 blocks (BDS: between 1.59 and 2.11) or has its eyes in water; damage alone
//! does not. Either starts the 100 ticks again.

use bevy_ecs::prelude::*;
use sapling_data::EYE_HEIGHT;

use super::Raised;
use crate::body::{Body, Liquid, liquid_in};
use crate::components::{Identity, InLayer, Layer, Position, Tick};
use crate::riding::{self, Seats};

const KIND: &str = "minecraft:parrot";
/// `start_delay`, `within_radius` and `mount_distance`.
const START_DELAY: u64 = 100;
const WITHIN: f32 = 16.0;
const MOUNTS: f32 = 2.0;
/// A fall at least this long throws the parrots off.
const THROWN_BY_FALL: f32 = 2.0;

/// The tick from which a parrot may look for a shoulder.
#[derive(Component)]
pub(super) struct Perch(u64);

fn waits(world: &mut World, parrot: Entity, now: u64) {
    world.entity_mut(parrot).insert(Perch(now + START_DELAY));
}

/// A tamed parrot looks for a shoulder and flies to it; false when it does not.
pub(super) fn steer(world: &mut World, parrot: Entity, layer: Entity, kind: &str) -> bool {
    if kind != KIND || !world.get::<Raised>(parrot).is_some_and(|raised| raised.is_tamed() && !raised.kept.sitting) {
        return false;
    }
    let now = world.resource::<Tick>().0;
    let Some(from) = world.get::<Perch>(parrot).map(|perch| perch.0) else {
        waits(world, parrot, now);
        return false;
    };
    let Some(at) = world.get::<Body>(parrot).map(|body| body.at).filter(|_| now >= from) else { return false };
    let distance = |feet: [f32; 3]| feet.iter().zip(at).map(|(a, b)| (a - b).powi(2)).sum::<f32>().sqrt();
    // Measured to the player's eyes, where its shoulders are and where the parrot flies.
    let eyes = |position: &Position| [position.feet[0], position.feet[1] + EYE_HEIGHT, position.feet[2]];
    let mut players: Vec<(Entity, [f32; 3])> =
        world.query_filtered::<(Entity, &Position, &InLayer), With<Identity>>().iter(world).filter(|(_, _, in_layer)| in_layer.0 == layer).map(|(player, position, _)| (player, eyes(position))).collect();
    players.retain(|(player, eyes)| distance(*eyes) <= WITHIN && riding::can_seat(world, parrot, *player).is_ok());
    let Some((player, eyes)) = players.into_iter().min_by(|a, b| distance(a.1).total_cmp(&distance(b.1))) else { return false };
    if distance(eyes) <= MOUNTS {
        return riding::seat(world, parrot, player).is_ok();
    }
    super::super::wings::fly_at(world, parrot, kind, eyes)
}

/// Lets the parrots off a player who has fallen far enough or has its eyes in water.
pub(super) fn tick(world: &mut World) {
    let carrying: Vec<(Entity, Vec<Entity>)> = world.query::<(Entity, &Seats)>().iter(world).filter(|(player, _)| world.get::<Identity>(*player).is_some()).map(|(player, seats)| (player, seats.0.clone())).collect();
    for (player, parrots) in carrying {
        if !thrown(world, player) {
            continue;
        }
        let now = world.resource::<Tick>().0;
        for parrot in parrots.into_iter().rev() {
            riding::let_off(world, parrot);
            waits(world, parrot, now);
        }
    }
}

fn thrown(world: &World, player: Entity) -> bool {
    if crate::vitals::fallen(world, player) >= THROWN_BY_FALL {
        return true;
    }
    let (Some(position), Some(layer)) = (world.get::<Position>(player), world.get::<InLayer>(player)) else { return false };
    let eyes = [position.feet[0], position.feet[1] + EYE_HEIGHT, position.feet[2]].map(|c| c.floor() as i32);
    world.get::<Layer>(layer.0).is_some_and(|level| liquid_in(&level.world, eyes, Liquid::Water).is_some())
}
