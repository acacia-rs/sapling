//! Torches, buttons and rails set where nothing holds them: BDS shows them breaking at once and
//! takes them out, leaving their item, a tick later; a lever, ladder, flower, carpet or wall sign
//! set so stays (research/captures/bds-unheld.txt).

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use super::super::queued::{self, Job};
use super::super::{level, send};

/// Name endings of the blocks that go.
const POPS: &[&str] = &["torch", "_button", "rail"];

/// The block at `pos`, when it is one of [`POPS`] with nothing holding it.
fn unheld(world: &World, layer: Entity, pos: BlockPos) -> Option<u32> {
    let level = level(world, layer).filter(|level| level.in_height(pos[1]))?;
    let id = level.block(pos[0], pos[1], pos[2]);
    let name = level.registry().get(id)?.name;
    (POPS.iter().any(|end| name.ends_with(end)) && super::lacks_support(world, layer, pos, id)).then_some(id)
}

/// Shows the block just set at `pos` breaking (a rail without particles), and queues its going,
/// when it is unheld.
pub(super) fn noticed(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(id) = unheld(world, layer, pos) else { return };
    let Some(level) = level(world, layer) else { return };
    if !level.registry().get(id).is_some_and(|state| state.name.ends_with("rail")) {
        let particles = send::destroy_particles(level.registry(), pos, id);
        send::to_viewers(world, layer, pos, &[particles]);
    }
    queued::after(world, layer, pos, 1, Job::Unheld);
}

/// [`noticed`] for each block beside `cells`, which a piston just took away: BDS's torch beside a
/// pushed block went a tick into the push, not at its end.
pub(in crate::blocks) fn around(world: &mut World, layer: Entity, cells: &[BlockPos]) {
    let mut beside: Vec<BlockPos> = cells.iter().flat_map(|cell| Face::ALL.map(|face| face.neighbour(*cell))).filter(|at| !cells.contains(at)).collect();
    beside.sort_unstable();
    beside.dedup();
    beside.into_iter().for_each(|at| noticed(world, layer, at));
}

/// The block [`noticed`] at `pos` goes, unless something came to hold it.
pub(in crate::blocks) fn popped(world: &mut World, layer: Entity, pos: BlockPos) {
    if unheld(world, layer, pos).is_some() {
        super::take(world, layer, pos, &super::super::loot::Breaker::UNAIDED, false);
        super::settle(world, layer, pos);
    }
}
