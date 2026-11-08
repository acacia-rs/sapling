//! What loses its hold when a block beside it changes: BDS shows it breaking in that tick and takes
//! it out, with its item, a tick later; what its going loosens is shown then and goes a tick after
//! (a lever, wall banner and vines in research/captures/bds-holds.txt, a lantern in bds-burnfloat.txt).

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use super::super::growth::vines;
use super::super::queued::{self, Job};
use super::super::{doors, ground, level, loot, send};

/// Shows the block at `pos` breaking (a rail not, as one set unheld: updates/unheld.rs) and queues
/// its going, once.
pub(super) fn noticed(world: &mut World, layer: Entity, pos: BlockPos) {
    if queued::is_waiting(world, layer, pos, Job::Loosened) {
        return;
    }
    let Some(level) = level(world, layer) else { return };
    let id = level.block(pos[0], pos[1], pos[2]);
    if !level.registry().get(id).is_some_and(|state| state.name.ends_with("rail")) {
        let particles = send::destroy_particles(level.registry(), pos, id);
        if !send::hold_break(world, layer, pos, id) {
            send::to_viewers(world, layer, pos, &[particles]);
        }
    }
    queued::after(world, layer, pos, 1, Job::Loosened);
}

/// Whether the block at `pos` has nothing left to hold it: a face it hangs or stands on that is not
/// solid (but its own other half), ground it may not stand on, or a vine with no face kept.
fn is_loose(world: &World, layer: Entity, pos: BlockPos) -> bool {
    let Some(level) = level(world, layer).filter(|level| level.in_height(pos[1])) else { return false };
    let (registry, id) = (level.registry(), level.block(pos[0], pos[1], pos[2]));
    if registry.get(id).is_none_or(|state| state.is_air()) {
        return false;
    }
    let own_half = doors::other_half(world, layer, pos);
    let held = registry.get(id).map_or("", |state| state.name);
    let holds = |[x, y, z]: BlockPos, face| level.in_height(y) && registry.get(level.block(x, y, z)).is_some_and(|state| super::holds(state, face, held));
    let unheld = Face::ALL.into_iter().any(|face| {
        let support = [0, 1, 2].map(|axis| pos[axis] - face.offset()[axis]);
        super::depended(registry, id, face) && own_half != Some(support) && !holds(support, face)
    });
    unheld || !ground::may_stand(level, pos, id) || ground::is_wrongly_lit(level, pos, id) || vines::loosened(level, pos) == Some(0)
}

/// [`noticed`] for what the blocks of `cells` held, which a piston just took away.
pub(in crate::blocks) fn around(world: &mut World, layer: Entity, cells: &[BlockPos]) {
    let mut beside: Vec<BlockPos> = cells.iter().flat_map(|cell| Face::ALL.map(|face| face.neighbour(*cell))).filter(|at| !cells.contains(at)).collect();
    beside.sort_unstable();
    beside.dedup();
    beside.retain(|at| is_loose(world, layer, *at));
    beside.into_iter().for_each(|at| noticed(world, layer, at));
}

/// The block [`noticed`] at `pos` goes, unless something came to hold it; it was shown breaking then.
pub(in crate::blocks) fn popped(world: &mut World, layer: Entity, pos: BlockPos) {
    if is_loose(world, layer, pos) {
        super::take(world, layer, pos, &loot::Breaker::UNAIDED, false);
        super::settle(world, layer, pos);
    }
}
