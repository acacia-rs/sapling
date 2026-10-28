//! A villager's bed (its pack's `minecraft:dweller`, `behavior.move_towards_dwelling_restriction`
//! and `behavior.sleep`): at its looks (jobs.rs) one without a bed claims the nearest free bed's
//! head as it does a job site; its schedule (schedule.rs) takes it home to it, and at bed time it
//! lies on it until its schedule wakes it.
// TODO: BDS's bed claim (a village POI), the sleeper's flags and pose and the bed's
// `occupied_bit` were not decompiled; a sleeper here rests on its bed's head. Babies claim none.

use bevy_ecs::prelude::*;
use sapling_world::World as Level;

use super::Trader;
use super::jobs::{self, Claimed};
use crate::body::Body;

const BED: &str = "minecraft:bed";
/// `minecraft:movement` 0.5 times `behavior.sleep`'s `speed_multiplier` 0.6.
const GOES: f32 = 0.3;
/// How near its bed's head a villager lies down.
const LIES: f32 = 0.6;

fn is_head(level: &Level, [x, y, z]: [i32; 3]) -> bool {
    level.registry().get(level.block(x, y, z)).and_then(|state| state.property("head_piece_bit")) == Some("1")
}

/// At a villager's look: it gives up a bed that went, and one without a bed claims one.
pub(super) fn decide(level: &Level, claimed: &mut Claimed, trader: &mut Trader, (entity, layer, feet, now): (Entity, Entity, [i32; 3], u64)) {
    match trader.bed {
        Some(bed) if jobs::block_at(level, bed) != Some(BED) => {
            claimed.sites.remove(&(layer, bed));
            trader.bed = None;
        }
        None if claimed.may_search(now, jobs::FOR_BEDS) => {
            let fits = |cell, name: &str| (name == BED && is_head(level, cell)).then_some(0);
            if let Some((bed, _)) = jobs::nearest(level, claimed, layer, feet, fits) {
                claimed.sites.insert((layer, bed), entity);
                trader.bed = Some(bed);
            }
        }
        _ => {}
    }
}

/// The middle of the top of the villager's bed, if it has one.
pub(super) fn own(world: &World, entity: Entity) -> Option<[f32; 3]> {
    let [x, y, z] = world.get::<Trader>(entity)?.bed?;
    Some([x as f32 + 0.5, y as f32 + 0.5625, z as f32 + 0.5])
}

/// Walks the villager to its bed at `speed`, and keeps it there: false without one.
pub(super) fn go_to(world: &mut World, entity: Entity, layer: Entity, body: &Body, speed: f32, near: f32) -> bool {
    let Some(bed) = own(world, entity) else { return false };
    if (bed[0] - body.at[0]).hypot(bed[2] - body.at[2]) <= near {
        super::super::legs::rest(world, entity);
    } else {
        super::super::hunting::approach(world, entity, layer, body, bed, speed, near);
    }
    true
}

/// At bed time the villager goes to bed and sleeps there: false without one.
pub(super) fn sleep(world: &mut World, entity: Entity, layer: Entity, body: &Body) -> bool {
    go_to(world, entity, layer, body, GOES, LIES)
}
