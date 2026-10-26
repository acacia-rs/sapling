//! Block ticks queued for later, for the blocks that wait a while after a signal or a touch (BDS's
//! `BlockTickingQueue`, docs/research/tick-queue.md): each job checks its block is still there.
// TODO: not saved with the column, as BDS's `PendingTicks` are: a restart drops what waits.

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use crate::components::Tick;

/// Jobs run in one tick at most; the rest wait for the next.
const MAX_PER_TICK: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Job {
    /// A dispenser, dropper or crafter fires.
    Fire,
    /// A crafter's `crafting` light goes out.
    Crafted,
    /// A target's signal ends.
    TargetOff,
    /// A lightning rod's signal ends.
    RodOff,
    /// A pressed tripwire looks for what is on it again.
    Tripwire,
    /// A sculk sensor or shrieker steps on to its next phase.
    Sculk,
}

#[derive(Resource, Default)]
pub(crate) struct Queued(Vec<(u64, Entity, BlockPos, Job)>);

/// Runs `job` on the block at `pos` `delay` ticks from now.
pub(crate) fn after(world: &mut World, layer: Entity, pos: BlockPos, delay: u64, job: Job) {
    let when = world.resource::<Tick>().0 + delay.max(1);
    world.get_resource_or_insert_with(Queued::default).0.push((when, layer, pos, job));
}

/// Drops what waits of `job` for the block at `pos`.
pub(crate) fn cancel(world: &mut World, layer: Entity, pos: BlockPos, job: Job) {
    if let Some(mut queued) = world.get_resource_mut::<Queued>() {
        queued.0.retain(|(_, at_layer, at, waiting)| (*at_layer, *at, *waiting) != (layer, pos, job));
    }
}

/// Runs the jobs that are due, oldest first.
pub(crate) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let Some(mut queued) = world.get_resource_mut::<Queued>().filter(|queued| queued.0.iter().any(|(when, ..)| *when <= now)) else { return };
    let (mut due, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut queued.0).into_iter().partition(|(when, ..)| *when <= now);
    queued.0 = waiting;
    due.sort_by_key(|(when, ..)| *when);
    if due.len() > MAX_PER_TICK {
        let later = due.split_off(MAX_PER_TICK);
        world.resource_mut::<Queued>().0.extend(later);
    }
    for (_, layer, pos, job) in due {
        run(world, layer, pos, job);
    }
}

fn run(world: &mut World, layer: Entity, pos: BlockPos, job: Job) {
    match job {
        Job::Fire => super::dispensers::fire(world, layer, pos),
        Job::Crafted => super::crafter::crafted(world, layer, pos),
        Job::TargetOff => super::target::ended(world, layer, pos),
        Job::RodOff => super::rods::ended(world, layer, pos),
        Job::Tripwire => super::tripwire::recheck(world, layer, pos),
        Job::Sculk => super::sculk::step(world, layer, pos),
    }
}
