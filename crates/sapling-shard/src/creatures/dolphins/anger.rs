//! A dolphin's anger, as BDS's three in `f14edolphin2` (research/captures/bds-f14edolphin2.txt;
//! the pack's `dolphin_angry`): one that takes a player for its prey passes it to every dolphin
//! within 16, each showing `ANGRY` from the tick after it took it. The hit one took it in the hit's
//! tick, those near in the next (each heard and shown angry a tick after that; sapling's
//! f14edolphin2 had them all at once). Its anger ends 25 s after it began or after its last bite,
//! whichever is later; it may lose its prey before then.

use bevy_ecs::prelude::*;
use acacia_proto::types::MetadataFlags1;

use super::{Dolphin, KIND};
use crate::body::Body;
use crate::components::{InLayer, NetId, Tick};
use crate::entities;

const SPREADS: f32 = 16.0;
const LASTS: u64 = 500;

/// A player a dolphin near took for its prey, and the tick it did: this one takes it at its next tick.
#[derive(Component)]
pub(super) struct Joins(Entity, u64);

/// The dolphin took `player` for its prey: it is angry from the next tick, and each dolphin within
/// 16 of `at` (`broadcast_anger`) takes `player` at its next tick ([`join`]).
pub(in crate::creatures) fn anger(world: &mut World, dolphin: Entity, layer: Entity, at: [f32; 3], player: Entity) {
    let now = world.resource::<Tick>().0;
    let Some(mut angered) = world.get_mut::<Dolphin>(dolphin) else { return };
    angered.angry = Some([now + 1, now + 1 + LASTS]);
    let near: Vec<Entity> = world
        .query::<(Entity, &InLayer, &Body, &Dolphin)>()
        .iter(world)
        .filter(|(other, in_layer, body, _)| *other != dolphin && in_layer.0 == layer && (0..3).map(|axis| (body.at[axis] - at[axis]).powi(2)).sum::<f32>() <= SPREADS * SPREADS)
        .map(|(other, ..)| other)
        .collect();
    for other in near {
        world.entity_mut(other).insert(Joins(player, now));
    }
}

/// The dolphin takes the prey one near passed it in an earlier tick.
pub(super) fn join(world: &mut World, dolphin: Entity, layer: Entity, body: &Body) {
    let now = world.resource::<Tick>().0;
    let Some(&Joins(player, since)) = world.get::<Joins>(dolphin) else { return };
    if now <= since {
        return;
    }
    world.entity_mut(dolphin).remove::<Joins>();
    let Some(id) = world.get::<NetId>(dolphin).map(|id| id.0) else { return };
    if world.get::<InLayer>(player).is_some_and(|on| on.0 == layer) {
        super::super::hunting::provoke(world, dolphin, layer, id, KIND, body, player);
    }
}

/// The dolphin bit its prey: its anger lasts 25 s from now.
pub(in crate::creatures) fn bit(world: &mut World, dolphin: Entity) {
    let now = world.resource::<Tick>().0;
    if let Some(mut bitten) = world.get_mut::<Dolphin>(dolphin) {
        if let Some([_, until]) = bitten.angry.as_mut() {
            *until = (*until).max(now + LASTS);
        }
    }
}

/// Shows the dolphin's `ANGRY` as its anger starts and ends.
pub(super) fn show(world: &mut World, dolphin: Entity, layer: Entity, body: &Body) {
    let now = world.resource::<Tick>().0;
    let Some(state) = world.get::<Dolphin>(dolphin) else { return };
    let angry = state.angry.is_some_and(|[from, until]| (from..until).contains(&now));
    if angry == state.shown {
        return;
    }
    let Some(id) = world.get::<NetId>(dolphin).map(|id| id.0) else { return };
    let mut state = world.get_mut::<Dolphin>(dolphin).expect("read above");
    state.shown = angry;
    if !angry {
        state.angry = None;
    }
    let flags = entities::mob_flags(world, dolphin, KIND, MetadataFlags1::ANGRY, angry);
    entities::tell_viewers(world, dolphin, layer, body.at.map(|c| c.floor() as i32), vec![crate::sync::flags_packet(id, flags)]);
}
