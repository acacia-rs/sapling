//! Block ticks queued for later, for the blocks that wait a while after a signal or a touch (BDS's
//! `BlockTickingQueue`, docs/research/tick-queue.md): each job checks its block is still there.
// What waits is saved with its column (pending.rs).

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
    /// A water source or bubble column becomes what the block under it asks for (bubbles.rs).
    Bubbles,
    /// A stalactite's drip reaches the cauldron under it (drips.rs).
    Drip,
    /// A lectern's page-turn pulse starts or ends (lectern.rs).
    Lectern,
    /// A command block runs (command_blocks/run.rs).
    Command,
    /// Coral out of water dies (coral.rs).
    Coral,
    /// A sponge soaks up water, or a wet one dries in the Nether (sponge.rs).
    Sponge,
}

#[derive(Resource, Default)]
pub(crate) struct Queued(Vec<(u64, Entity, BlockPos, Job)>);

/// Runs `job` on the block at `pos` `delay` ticks from now.
pub(crate) fn after(world: &mut World, layer: Entity, pos: BlockPos, delay: u64, job: Job) {
    let when = world.resource::<Tick>().0 + delay.max(1);
    world.get_resource_or_insert_with(Queued::default).0.push((when, layer, pos, job));
}

/// Runs `job` on the block at `pos` later in this tick, or in the next where this tick's jobs ran.
pub(crate) fn this_tick(world: &mut World, layer: Entity, pos: BlockPos, job: Job) {
    let now = world.resource::<Tick>().0;
    world.get_resource_or_insert_with(Queued::default).0.push((now, layer, pos, job));
}

/// Drops what waits of `job` for the block at `pos`.
pub(crate) fn cancel(world: &mut World, layer: Entity, pos: BlockPos, job: Job) {
    if let Some(mut queued) = world.get_resource_mut::<Queued>() {
        queued.0.retain(|(_, at_layer, at, waiting)| (*at_layer, *at, *waiting) != (layer, pos, job));
    }
}

/// What waits, for its column to keep (pending.rs): each job's cell and when it is due.
pub(super) fn waiting(world: &World) -> Vec<(Entity, BlockPos, u64)> {
    world.get_resource::<Queued>().map(|queued| queued.0.iter().map(|&(when, layer, pos, _)| (layer, pos, when)).collect()).unwrap_or_default()
}

/// Drops what waits where `saved` says its column kept it and is gone.
pub(super) fn forget(world: &mut World, saved: &dyn Fn(Entity, BlockPos) -> bool) {
    if let Some(mut queued) = world.get_resource_mut::<Queued>() {
        queued.0.retain(|&(_, layer, pos, _)| !saved(layer, pos));
    }
}

/// The job the block `name` with `properties` waits for, to queue again what a column kept.
pub(super) fn job_of(name: &str, properties: &str) -> Option<Job> {
    Some(match name.strip_prefix("minecraft:")? {
        // A lit crafter waits for its light to go out.
        "crafter" if properties.split(',').any(|pair| pair == "crafting=1") => Job::Crafted,
        "dispenser" | "dropper" | "crafter" => Job::Fire,
        "target" => Job::TargetOff,
        "lightning_rod" => Job::RodOff,
        "tripwire" | "trip_wire" => Job::Tripwire,
        "sculk_sensor" | "calibrated_sculk_sensor" | "sculk_shrieker" => Job::Sculk,
        "bubble_column" => Job::Bubbles,
        "cauldron" => Job::Drip,
        "lectern" => Job::Lectern,
        "command_block" | "repeating_command_block" | "chain_command_block" => Job::Command,
        name if super::coral::is_live(&format!("minecraft:{name}")) => Job::Coral,
        _ => return None,
    })
}

/// [`after`], unless the same job already waits at that tick: a column saved and loaded again
/// before its jobs ran brings them back a second time.
pub(super) fn again(world: &mut World, layer: Entity, pos: BlockPos, delay: u64, job: Job) {
    let when = world.resource::<Tick>().0 + delay.max(1);
    let queued = world.get_resource::<Queued>().is_some_and(|queued| queued.0.contains(&(when, layer, pos, job)));
    if !queued {
        after(world, layer, pos, delay, job);
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
        Job::Bubbles => super::bubbles::settle(world, layer, pos),
        Job::Drip => super::drips::fill(world, layer, pos),
        Job::Lectern => super::lectern::pulse(world, layer, pos),
        Job::Command => super::command_blocks::tick(world, layer, pos),
        Job::Coral => super::coral::die(world, layer, pos),
        Job::Sponge => super::sponge::ticked(world, layer, pos),
    }
}
