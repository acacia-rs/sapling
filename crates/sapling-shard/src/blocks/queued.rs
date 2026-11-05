//! Block ticks queued for later (BDS's `BlockTickingQueue`, docs/research/tick-queue.md): liquids
//! that move (liquid.rs) and the blocks that wait a while after a signal or a touch share one queue
//! per column (chunk_ticks.rs) and its 100 a tick, as BDS's chunk keeps one `mTickQueue` for both.
//! Each job checks its block is still there.
// What waits is saved with its column (pending.rs).

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::ChunkPos;

use super::chunk_ticks::ChunkTicks;
use super::liquid::{self, Liquid};
use crate::chunks::UNLOAD_INTERVAL;
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

/// What a queued tick is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Ticked {
    /// The liquid in the cell moves (liquid.rs).
    Flow(Liquid),
    Job(Job),
}

impl From<Job> for Ticked {
    fn from(job: Job) -> Self {
        Ticked::Job(job)
    }
}

#[derive(Resource, Default)]
pub(crate) struct Queued(ChunkTicks<Ticked>);

/// The last tick each column ticked, while loaded: BDS dates a queued tick from its chunk queue's
/// `mCurrentTick`, which stands still while the chunk does not tick (bds-e2stale.txt).
#[derive(Resource, Default)]
struct Clocks(HashMap<(Entity, ChunkPos), u64>);

/// The tick a tick queued at `pos` counts its delay from: now, or the last tick of a column that
/// has stopped ticking. A column never seen ticking counts from now.
pub(super) fn clock(world: &World, layer: Entity, [x, _, z]: BlockPos) -> u64 {
    let now = world.resource::<Tick>().0;
    match world.get_resource::<Clocks>().and_then(|clocks| clocks.0.get(&(layer, ChunkPos::of_block(x, z)))) {
        Some(&ticked) if ticked + 1 < now => ticked,
        _ => now,
    }
}

/// Dates the columns that tick now; drops those no longer loaded.
fn wind_clocks(world: &mut World, now: u64) {
    let ticking = crate::ticking::ticked_columns(world);
    world.resource_scope(|world, mut clocks: Mut<Clocks>| {
        clocks.0.extend(ticking.into_iter().map(|column| (column, now)));
        if now % UNLOAD_INTERVAL == 0 {
            clocks.0.retain(|&(layer, column), _| super::level(world, layer).is_some_and(|level| level.is_loaded(column)));
        }
    });
}

pub(super) fn add(world: &mut World, layer: Entity, pos: BlockPos, due: u64, ticked: Ticked) {
    world.get_resource_or_insert_with(Queued::default).0.add(layer, pos, due, ticked);
}

/// Whether `ticked` waits for the cell at `pos`, due at `due` if given.
pub(super) fn has(world: &World, layer: Entity, pos: BlockPos, due: Option<u64>, ticked: Ticked) -> bool {
    world.get_resource::<Queued>().is_some_and(|queued| queued.0.has(layer, pos, due, |waiting| *waiting == ticked))
}

/// Runs `job` on the block at `pos` `delay` ticks from now.
pub(crate) fn after(world: &mut World, layer: Entity, pos: BlockPos, delay: u64, job: Job) {
    let due = clock(world, layer, pos) + delay.max(1);
    add(world, layer, pos, due, Ticked::Job(job));
}

/// Runs `job` on the block at `pos` later in this tick, or in the next where its column's jobs ran.
pub(crate) fn this_tick(world: &mut World, layer: Entity, pos: BlockPos, job: Job) {
    let now = world.resource::<Tick>().0;
    add(world, layer, pos, now, Ticked::Job(job));
}

/// Whether `job` waits for the block at `pos`.
pub(crate) fn is_waiting(world: &World, layer: Entity, pos: BlockPos, job: Job) -> bool {
    has(world, layer, pos, None, Ticked::Job(job))
}

/// Drops what waits of `job` for the block at `pos`.
pub(crate) fn cancel(world: &mut World, layer: Entity, pos: BlockPos, job: Job) {
    if let Some(mut queued) = world.get_resource_mut::<Queued>() {
        queued.0.remove(layer, pos, |waiting| *waiting == Ticked::Job(job));
    }
}

/// What waits, liquids' ticks and jobs, for its column to keep (pending.rs): each tick's cell and
/// when it is due.
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

/// Queues `ticked` at `pos` in `delay` ticks, unless the same waits for that tick: a column saved
/// and loaded again before its ticks ran brings them back a second time.
pub(super) fn again(world: &mut World, layer: Entity, pos: BlockPos, delay: u64, ticked: impl Into<Ticked>) {
    let (due, ticked) = (world.resource::<Tick>().0 + delay.max(1), ticked.into());
    if !has(world, layer, pos, Some(due), ticked) {
        add(world, layer, pos, due, ticked);
    }
}

/// Runs the ticks that are due, column by column where the column ticks ([`Region::ticks`]).
pub(crate) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    world.init_resource::<Clocks>();
    wind_clocks(world, now);
    let Some(columns) = world.get_resource_mut::<Queued>().map(|mut queued| queued.0.columns_due(now)).filter(|columns| !columns.is_empty()) else { return };
    let region = Region::of(world);
    for (layer, column) in columns {
        if !region.ticks(world, layer, column) {
            continue;
        }
        let due = world.resource_mut::<Queued>().0.take(layer, column, now);
        for (pos, ticked) in due {
            match held_back(world, layer, pos) {
                Some(true) => add(world, layer, pos, now, ticked),
                Some(false) => {}
                None => run(world, layer, pos, ticked),
            }
        }
    }
}

/// `tickPendingTicks` (1416042f0): a tick within 8 blocks of a column not loaded is not run but
/// queued again for the next game tick (`Some(true)`), or dropped where its own column is gone.
fn held_back(world: &World, layer: Entity, [x, _, z]: BlockPos) -> Option<bool> {
    let level = super::level(world, layer)?;
    let loaded = |dx: i32, dz: i32| level.is_loaded(ChunkPos::of_block(x + dx, z + dz));
    let around = [(-8, -8), (8, -8), (-8, 8), (8, 8)].into_iter().all(|(dx, dz)| loaded(dx, dz));
    (!around).then(|| loaded(0, 0))
}

fn run(world: &mut World, layer: Entity, pos: BlockPos, ticked: Ticked) {
    let job = match ticked {
        Ticked::Flow(liquid) => return liquid::ticked(world, layer, pos, liquid),
        Ticked::Job(job) => job,
    };
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
