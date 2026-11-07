//! An allay collecting what matches its hand (`behavior.pickup_items`, priority 2: `max_dist` and
//! `search_height` 32, `goal_radius` 2.2, `speed_multiplier` 6, `pickup_same_items_as_in_hand`), as
//! BDS's did (research/captures/bds-ap3give.txt, bds-ap3note.txt): the tick after a matching item
//! lies within 32 it flies at the nearest, and once its move leaves it within 2.2 of one it takes it
//! (`TakeItemEntity`, its hand unchanged). It leaves the rest alone, and takes nothing while it rests
//! after a throw (giving.rs) or its stack is full.

use acacia_proto::packets::TakeItemEntity;
use bevy_ecs::prelude::*;
use sapling_world::ItemStack;

use super::{ALLAY, Errands, held_by_allay};
use crate::body::Body;
use crate::components::{NetId, Tick};
use crate::creatures::wings;
use crate::drops;

const SEES: f32 = 32.0;
const REACH: f32 = 2.2;
const SPEED: f32 = 6.0;

/// How many of `hand`'s kind it may take yet; none while it rests.
fn room(world: &World, allay: Entity, hand: &ItemStack) -> u8 {
    let Some(errands) = world.get::<Errands>(allay).filter(|errands| world.resource::<Tick>().0 >= errands.rests_until) else { return 0 };
    let carried = errands.carried.as_ref().filter(|carried| carried.name == hand.name).map_or(0, |carried| carried.count);
    let other = errands.carried.as_ref().is_some_and(|carried| carried.name != hand.name);
    if other { 0 } else { sapling_data::max_stack(&hand.name).saturating_sub(carried) }
}

/// The matching items lying within its sight, nearest first, with where they lie.
fn matching(world: &mut World, layer: Entity, body: &Body, hand: &ItemStack) -> Vec<(Entity, ItemStack, f32)> {
    let lying = drops::lying_in(world, layer, body.at.map(|c| c - SEES), body.at.map(|c| c + SEES));
    let mut found: Vec<_> = lying
        .into_iter()
        .filter(|(item, stack)| stack.name == hand.name && drops::mobs_may_take(world, *item))
        .filter_map(|(item, stack)| world.get::<Body>(item).map(|lies| (item, stack, super::super::distance(lies.at, body.at))))
        .collect();
    found.sort_by(|a, b| a.2.total_cmp(&b.2));
    found
}

/// Before its move: it flies at the nearest matching item. False where it may take none or none lies near.
pub(super) fn steer(world: &mut World, allay: Entity, layer: Entity, body: &Body, hand: &ItemStack) -> bool {
    if room(world, allay, hand) == 0 {
        return false;
    }
    let Some(at) = matching(world, layer, body, hand).first().and_then(|(item, ..)| world.get::<Body>(*item)).map(|lies| lies.at) else { return false };
    // At the middle of the item's cell: aimed at the item itself it sank to the floor and stuck there.
    wings::fly_at_times(world, allay, ALLAY, [at[0], at[1].floor() + 0.5, at[2]], SPEED)
}

/// After its move: it takes up the nearest matching item within reach.
pub(in crate::creatures) fn take_up(world: &mut World, allay: Entity, layer: Entity, id: u64, kind: &str, body: &Body) {
    let Some(hand) = held_by_allay(world, allay).filter(|_| kind == ALLAY) else { return };
    let room = room(world, allay, &hand);
    if room == 0 {
        return;
    }
    let Some((item, stack, _)) = matching(world, layer, body, &hand).into_iter().next().filter(|(.., far)| *far <= REACH) else { return };
    let (Some(&NetId(item_id)), Some(lies)) = (world.get::<NetId>(item), world.get::<Body>(item).copied()) else { return };
    let taken = stack.count.min(room);
    let told = sapling_net::encode(&TakeItemEntity { runtime_entity_id: item_id, target: id as u32 });
    crate::blocks::to_viewers(world, layer, lies.at.map(|c| c.floor() as i32), &[told]);
    drops::take(world, item, taken);
    let mut errands = super::errands(world, allay);
    let had = errands.carried.as_ref().map_or(0, |carried| carried.count);
    errands.carried = Some(ItemStack { count: had + taken, ..stack });
    super::keep_carried(world, allay);
}
