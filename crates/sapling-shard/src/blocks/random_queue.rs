//! BDS's random-tick queue (`LevelChunk` +0x138, `mRandomTickQueue`), which kelp, the nether vines and
//! fire wait on: a wait is counted in the queue's steps, and a ticking chunk runs `randomTickSpeed` steps
//! a game tick (`LevelChunk::tick` 0x1409f98a0), so all of it goes faster with the rule. A column that
//! does not tick holds what waits in it (`ticking::Region::hold`, by [`speed`]).

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use sapling_data::RuleValue;

use super::level;
use crate::components::Tick;

const RULE: &str = "randomTickSpeed";

/// Each world's steps so far, and the game tick they were counted to.
#[derive(Resource, Default)]
pub(crate) struct Steps(HashMap<Entity, (u64, u64)>);

/// The steps the world's queue runs a game tick: its `randomTickSpeed`.
pub(crate) fn speed(world: &World, layer: Entity) -> u64 {
    match level(world, layer).and_then(|level| level.level().rules.get(RULE)) {
        Some((_, RuleValue::Int(speed))) => speed.max(0) as u64,
        _ => 0,
    }
}

/// The world's queue clock now: the steps counted, and on from there at the speed it has now.
pub(crate) fn peek(world: &World, layer: Entity) -> u64 {
    let tick = world.resource::<Tick>().0;
    let (counted_to, so_far) = world.get_resource::<Steps>().and_then(|steps| steps.0.get(&layer).copied()).unwrap_or((tick, 0));
    so_far + speed(world, layer) * tick.saturating_sub(counted_to)
}

/// [`peek`], keeping the count, so that a later change of the rule counts from now on.
pub(crate) fn now(world: &mut World, layer: Entity) -> u64 {
    let (tick, steps) = (world.resource::<Tick>().0, peek(world, layer));
    world.get_resource_or_insert_with(Steps::default).0.insert(layer, (tick, steps));
    steps
}

/// The game ticks from now until `due` on the world's queue at the speed it has now, for a column
/// to keep (pending.rs); a queue that does not run keeps it a tick off.
pub(crate) fn ticks_until(world: &World, layer: Entity, due: u64) -> u64 {
    due.saturating_sub(peek(world, layer)).div_ceil(speed(world, layer).max(1))
}
