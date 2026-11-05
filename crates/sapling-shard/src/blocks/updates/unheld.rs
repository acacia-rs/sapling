//! Torches, buttons and rails set where nothing holds them: BDS shows them breaking at once and
//! takes them out, leaving their item, a tick later; a lever, ladder, flower, carpet or wall sign
//! set so stays (research/captures/bds-unheld.txt). A small dripleaf set off its ground shows both
//! halves breaking and goes whole (bds-t5holds.txt).

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::super::queued::{self, Job};
use super::super::{doors, level, send};

/// Name endings of the blocks that go.
const POPS: &[&str] = &["torch", "_button", "rail", "small_dripleaf_block"];

/// The block at `pos`, when it is one of [`POPS`] with nothing holding it.
fn unheld(world: &World, layer: Entity, pos: BlockPos) -> Option<u32> {
    let level = level(world, layer).filter(|level| level.in_height(pos[1]))?;
    let id = level.block(pos[0], pos[1], pos[2]);
    let state = level.registry().get(id)?;
    // A dripleaf's upper half set alone stayed till a neighbour changed (bds-t5holds.txt).
    let is_upper = state.property("upper_block_bit") == Some("1");
    (POPS.iter().any(|end| state.name.ends_with(end)) && !is_upper && super::lacks_support(world, layer, pos, id)).then_some(id)
}

/// Shows the block just set at `pos` breaking (a rail without particles), and queues its going,
/// when it is unheld.
pub(super) fn noticed(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(id) = unheld(world, layer, pos) else { return };
    let Some(level) = level(world, layer) else { return };
    if !level.registry().get(id).is_some_and(|state| state.name.ends_with("rail")) {
        let other = doors::other_half(world, layer, pos).map(|cell| (cell, level.block(cell[0], cell[1], cell[2])));
        let particles: Vec<_> = [Some((pos, id)), other].into_iter().flatten().map(|(cell, id)| send::destroy_particles(level.registry(), cell, id)).collect();
        send::to_viewers(world, layer, pos, &particles);
    }
    queued::after(world, layer, pos, 1, Job::Unheld);
}

/// The block [`noticed`] at `pos` goes, unless something came to hold it; both halves of a
/// two-cell one in one UpdateSubchunkBlocks.
pub(in crate::blocks) fn popped(world: &mut World, layer: Entity, pos: BlockPos) {
    if unheld(world, layer, pos).is_none() {
        return;
    }
    let take = |world: &mut World| super::take(world, layer, pos, &super::super::loot::Breaker::UNAIDED, false);
    match doors::other_half(world, layer, pos) {
        Some(_) => send::batched(world, take),
        None => take(world),
    }
    super::settle(world, layer, pos);
}
