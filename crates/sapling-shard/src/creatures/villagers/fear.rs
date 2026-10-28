//! What a villager fears (its pack): `behavior.avoid_mob_type` walks it away from a zombie,
//! zombie villager, illager, vex or zoglin within 8; `behavior.hide` sends it to its bed for 30 s
//! once a bell rings near it. BDS's bell ring (0x141f1c400) marks every actor with
//! `minecraft:hide` in the bell's box grown 32 across and 12 up, then rests 300 ticks; a bell a
//! villager rings in a raid (`Village::tick`) marks none.

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::super::{Creature, Dying};
use super::Trader;
use crate::body::Body;
use crate::components::{InLayer, Tick};
use crate::entities::Kind;

const FEARED: [&str; 5] = ["zombie", "zombie_villager", "illager", "vex", "zoglin"];
const FEARS_WITHIN: f32 = 8.0;
/// `minecraft:movement` 0.5 times each goal's `speed_multiplier`: avoid 0.6, hide 0.8.
const FLEES: f32 = 0.3;
const HIDES: f32 = 0.4;
const HIDES_FOR: u64 = 600;
const HEARD_ACROSS: f32 = 32.0;
const HEARD_UP: f32 = 12.0;
const BELL_RESTS: u64 = 300;
/// How near its bed a hiding villager stops.
const HIDDEN: f32 = 1.0;

/// The tick a villager hides until.
#[derive(Component)]
pub(super) struct Hiding(u64);

/// Each bell's tick it alarms again from, by world and cell.
#[derive(Resource, Default)]
struct Rested(HashMap<(Entity, BlockPos), u64>);

/// A bell at `pos` of `layer` rang: the villagers near it hide, unless it rang lately.
pub(crate) fn alarm(world: &mut World, layer: Entity, pos: BlockPos) {
    let now = world.resource::<Tick>().0;
    let mut rested = world.remove_resource::<Rested>().unwrap_or_default();
    rested.0.retain(|_, until| *until > now);
    let rang = rested.0.insert((layer, pos), now + BELL_RESTS).is_some();
    world.insert_resource(rested);
    if rang {
        return;
    }
    let heard = |at: [f32; 3]| {
        let [x, y, z] = pos.map(|c| c as f32);
        (x - HEARD_ACROSS..=x + 1.0 + HEARD_ACROSS).contains(&at[0]) && (y - HEARD_UP..=y + 1.0 + HEARD_UP).contains(&at[1]) && (z - HEARD_ACROSS..=z + 1.0 + HEARD_ACROSS).contains(&at[2])
    };
    let hiding: Vec<Entity> = world
        .query_filtered::<(Entity, &InLayer, &Body, &Trader), Without<Dying>>()
        .iter(world)
        .filter(|(_, on, body, trader)| on.0 == layer && trader.leaves_at.is_none() && heard(body.at))
        .map(|(villager, ..)| villager)
        .collect();
    for villager in hiding {
        world.entity_mut(villager).insert(Hiding(now + HIDES_FOR));
    }
}

/// A hiding villager goes to its bed: false once it is done hiding, or has none.
pub(super) fn hides(world: &mut World, entity: Entity, layer: Entity, body: &Body) -> bool {
    let Some(until) = world.get::<Hiding>(entity).map(|hiding| hiding.0) else { return false };
    if world.resource::<Tick>().0 >= until {
        world.entity_mut(entity).remove::<Hiding>();
        return false;
    }
    super::beds::go_to(world, entity, layer, body, HIDES, HIDDEN)
}

/// A villager walks away from the nearest mob it fears: false with none near.
pub(super) fn flees(world: &mut World, entity: Entity, layer: Entity, body: &Body) -> bool {
    let off = |at: [f32; 3]| (at[0] - body.at[0]).hypot(at[2] - body.at[2]);
    let feared = world
        .query_filtered::<(&InLayer, &Kind, &Body), (With<Creature>, Without<Dying>)>()
        .iter(world)
        .filter(|(on, _, at)| on.0 == layer && (at.at[1] - body.at[1]).abs() <= FEARS_WITHIN && off(at.at) <= FEARS_WITHIN)
        .filter(|(_, kind, _)| sapling_data::mob_families(&kind.0).iter().any(|family| FEARED.contains(family)))
        .map(|(_, _, at)| at.at)
        .min_by(|a, b| off(*a).total_cmp(&off(*b)));
    let Some(from) = feared else { return false };
    let (dx, dz) = (body.at[0] - from[0], body.at[2] - from[2]);
    let length = dx.hypot(dz).max(0.01);
    let to = [body.at[0] + dx / length * FEARS_WITHIN, body.at[2] + dz / length * FEARS_WITHIN];
    super::super::legs::walk(world, entity, layer, body, to, FLEES, 0.0);
    true
}
