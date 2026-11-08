//! A polar bear rears before its hit (its pack's `behavior.stomp_attack`, the Java game's
//! `PolarBearMeleeAttackGoal`): within twice its reach of its prey it stands for the last 10 ticks of
//! each 20 before it may hit, and drops at the hit or as the 20 run out, when, its prey out of
//! reach, it waits another 20. BDS's showed `REARING` 10
//! ticks before each hit and dropped it with the swing, and between two hits 40 ticks apart reared
//! 10 ticks more, dropped 20 before the hit (research/captures/bds-provoke2.txt).

use acacia_proto::types::MetadataFlags1;
use bevy_ecs::prelude::*;

use super::{Hunter, REACH};
use crate::body::Body;
use crate::components::Tick;
use crate::entities;

const BEAR: &str = "minecraft:polar_bear";
const STANDS: u64 = 10;
const BEAT: u64 = 20;

/// Whether the bear is shown rearing.
#[derive(Component)]
pub(super) struct Rearing;

/// After its hit, if any: the bear stands or drops as above, which its viewers are told.
pub(super) fn rear(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body) {
    if kind != BEAR {
        return;
    }
    let now = world.resource::<Tick>().0;
    let Some((prey, ready)) = world.get::<Hunter>(entity).and_then(|hunter| Some((hunter.prey?, hunter.ready))) else { return set(world, entity, layer, id, kind, body, false) };
    let (Some(feet), size) = (super::feet_of(world, prey), super::prey::size_of(world, prey)) else { return };
    let stomps = 2.0 * ((body.size[0] + size[0]) / 2.0 + REACH);
    let near = (feet[0] - body.at[0]).abs() < stomps && (feet[2] - body.at[2]).abs() < stomps;
    // Ready and near but not struck this tick: it waits another beat, as the Java goal resets its cooldown.
    let ready = if near && now >= ready { now + BEAT } else { ready };
    if let Some(mut hunter) = world.get_mut::<Hunter>(entity) {
        hunter.ready = ready;
    }
    let stands = near && now < ready && (1..=STANDS).contains(&((ready - now) % BEAT));
    set(world, entity, layer, id, kind, body, stands);
}

fn set(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body, stands: bool) {
    if world.get::<Rearing>(entity).is_some() == stands {
        return;
    }
    match stands {
        true => world.entity_mut(entity).insert(Rearing),
        false => world.entity_mut(entity).remove::<Rearing>(),
    };
    let flags = entities::mob_flags(world, entity, kind, MetadataFlags1::REARING, stands);
    entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), vec![sapling_net::encode(&sapling_data::mob_flags_set(id, flags))]);
}
