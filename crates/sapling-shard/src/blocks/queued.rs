//! Block ticks queued for later, for the blocks that wait a while after a signal or a touch (BDS's
//! `BlockTickingQueue`, docs/research/tick-queue.md), in their columns' queues (chunk_ticks.rs):
//! each job checks its block is still there.
// What waits is saved with its column (pending.rs).

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::chunk_ticks::ChunkTicks;
use crate::components::Tick;
use crate::ticking::Region;

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
    /// A lectern turned to a page past 0 is shown again (lectern.rs).
    LecternShown,
    /// A command block runs (command_blocks/run.rs).
    Command,
    /// Coral out of water dies (coral.rs).
    Coral,
    /// A sponge soaks up water, or a wet one dries in the Nether (sponge.rs).
    Sponge,
    /// A big dripleaf tilts on, or back up (dripleaf.rs).
    Dripleaf,
    /// A turtle egg, frogspawn or sniffer egg cracks or hatches (eggs.rs).
    Hatch,
    /// A torch, button or rail set with nothing holding it goes (updates/unheld.rs).
    Unheld,
    /// What lost its hold when a block beside it changed goes (updates/loosened.rs).
    Loosened,
    /// A chest whose other half went stands alone (containers/pairs.rs).
    Unpaired,
    /// TNT a burning shot met is primed (tnt.rs).
    Ignite,
}

#[derive(Resource, Default)]
pub(crate) struct Queued(ChunkTicks<Job>);

fn add(world: &mut World, layer: Entity, pos: BlockPos, due: u64, job: Job) {
    world.get_resource_or_insert_with(Queued::default).0.add(layer, pos, due, job);
}

/// Runs `job` on the block at `pos` `delay` ticks from now.
pub(crate) fn after(world: &mut World, layer: Entity, pos: BlockPos, delay: u64, job: Job) {
    let due = world.resource::<Tick>().0 + delay.max(1);
    add(world, layer, pos, due, job);
}

/// Runs `job` on the block at `pos` later in this tick, or in the next where its column's jobs ran.
pub(crate) fn this_tick(world: &mut World, layer: Entity, pos: BlockPos, job: Job) {
    let now = world.resource::<Tick>().0;
    add(world, layer, pos, now, job);
}

/// Whether `job` waits for the block at `pos`.
pub(crate) fn is_waiting(world: &World, layer: Entity, pos: BlockPos, job: Job) -> bool {
    world.get_resource::<Queued>().is_some_and(|queued| queued.0.has(layer, pos, None, |waiting| *waiting == job))
}

/// Drops what waits of `job` for the block at `pos`.
pub(crate) fn cancel(world: &mut World, layer: Entity, pos: BlockPos, job: Job) {
    if let Some(mut queued) = world.get_resource_mut::<Queued>() {
        queued.0.remove(layer, pos, |waiting| *waiting == job);
    }
}

/// What waits, for its column to keep (pending.rs): each job's cell and when it is due.
pub(super) fn waiting(world: &World) -> Vec<(Entity, BlockPos, u64)> {
    world.get_resource::<Queued>().map(|queued| queued.0.waiting().into_iter().map(|(layer, pos, due, _)| (layer, pos, due)).collect()).unwrap_or_default()
}

/// Drops what waits where `saved` says its column kept it and is gone.
pub(super) fn forget(world: &mut World, saved: &dyn Fn(Entity, BlockPos) -> bool) {
    if let Some(mut queued) = world.get_resource_mut::<Queued>() {
        queued.0.forget(saved);
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
        "big_dripleaf" if !properties.split(',').any(|pair| pair == "big_dripleaf_tilt=none") => Job::Dripleaf,
        "command_block" | "repeating_command_block" | "chain_command_block" => Job::Command,
        name if super::coral::is_live(&format!("minecraft:{name}")) => Job::Coral,
        name if super::eggs::is_egg(&format!("minecraft:{name}")) => Job::Hatch,
        _ => return None,
    })
}

/// [`after`], unless the same job already waits at that tick: a column saved and loaded again
/// before its jobs ran brings them back a second time.
pub(super) fn again(world: &mut World, layer: Entity, pos: BlockPos, delay: u64, job: Job) {
    let due = world.resource::<Tick>().0 + delay.max(1);
    let queued = world.get_resource::<Queued>().is_some_and(|queued| queued.0.has(layer, pos, Some(due), |waiting| *waiting == job));
    if !queued {
        add(world, layer, pos, due, job);
    }
}

/// Runs the jobs that are due, column by column where the column ticks ([`Region::ticks`]).
pub(crate) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let Some(columns) = world.get_resource_mut::<Queued>().map(|mut queued| queued.0.columns_due(now)).filter(|columns| !columns.is_empty()) else { return };
    let region = Region::of(world);
    for (layer, column) in columns {
        if !region.ticks(world, layer, column) {
            continue;
        }
        let due = world.resource_mut::<Queued>().0.take(layer, column, now);
        for (pos, job) in due {
            run(world, layer, pos, job);
        }
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
        Job::LecternShown => super::lectern::reshow(world, layer, pos),
        Job::Command => super::command_blocks::tick(world, layer, pos),
        Job::Coral => super::coral::die(world, layer, pos),
        Job::Sponge => super::sponge::ticked(world, layer, pos),
        Job::Dripleaf => super::dripleaf::step(world, layer, pos),
        Job::Hatch => super::eggs::ticked(world, layer, pos),
        Job::Unheld => super::updates::unheld::popped(world, layer, pos),
        Job::Loosened => super::updates::loosened::popped(world, layer, pos),
        Job::Unpaired => crate::containers::stand_alone(world, layer, pos),
        Job::Ignite => crate::tnt::ignited(world, layer, pos),
    }
}
