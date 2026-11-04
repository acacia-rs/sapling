//! The order hoppers move in within a tick, as BDS's `HopperBlockActor::_tick` (1457f82a0) recurses
//! (docs/research/hoppers.md, "Tick"): a hopper whose move is due first ticks the hoppers that could
//! pull what it moves, the one under it, the one under the cell it faces and the one under that
//! chest's other half, in an order two draws shuffle, at most 50 deep. Each moves once a tick.

use std::collections::HashSet;

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use crate::blocks::Dice;

/// How deep BDS's tick entry (1457f81e0) lets the recursion go.
const DEPTH: u32 = 50;

/// The shuffle's own dice: drawing from the shard's would shift every later roll.
#[derive(Resource, Default)]
struct Shuffles(Dice);

/// `due`, sorted, put in the order BDS moves them: each after the due hoppers it ticks first.
pub(super) fn ordered(world: &mut World, due: &[(Entity, BlockPos)]) -> Vec<(Entity, BlockPos)> {
    let due_set: HashSet<(Entity, BlockPos)> = due.iter().copied().collect();
    let mut walk = Walk { due: due_set, seen: HashSet::new(), out: Vec::new() };
    for hopper in due {
        walk.visit(world, *hopper, DEPTH);
    }
    walk.out
}

struct Walk {
    due: HashSet<(Entity, BlockPos)>,
    seen: HashSet<(Entity, BlockPos)>,
    out: Vec<(Entity, BlockPos)>,
}

impl Walk {
    fn visit(&mut self, world: &mut World, hopper: (Entity, BlockPos), depth: u32) {
        // A hopper not due stops there: BDS returns before recursing when its cooldown runs.
        if !self.seen.insert(hopper) || !self.due.contains(&hopper) {
            return;
        }
        if depth >= 1 {
            for next in shuffled(world, downstream(world, hopper)) {
                self.visit(world, next, depth - 1);
            }
        }
        self.out.push(hopper);
    }
}

/// The hopper under `pos`, the one under the cell it faces, and the one under that cell's other
/// chest half; `None` where there is no hopper.
fn downstream(world: &World, (layer, pos): (Entity, BlockPos)) -> [Option<(Entity, BlockPos)>; 3] {
    let hopper = |at: BlockPos| super::is_watched(world, layer, at).then_some((layer, at));
    let below = |at: BlockPos| Face::Down.neighbour(at);
    let target = super::facing(world, layer, pos).map(|facing| facing.neighbour(pos));
    let partner = target.and_then(|target| crate::containers::chest_halves(world, layer, target).into_iter().find(|half| *half != target));
    [hopper(below(pos)), target.and_then(|target| hopper(below(target))), partner.and_then(|half| hopper(below(half)))]
}

/// BDS's shuffle: an even first draw swaps the first two, then a second draw `k` of three swaps
/// the `k`th with the last unless it is the last.
fn shuffled(world: &mut World, mut next: [Option<(Entity, BlockPos)>; 3]) -> Vec<(Entity, BlockPos)> {
    let mut shuffles = world.get_resource_or_insert_with(Shuffles::default);
    if shuffles.0.roll(2) == 0 {
        next.swap(0, 1);
    }
    next.swap(shuffles.0.roll(3) as usize, 2);
    next.into_iter().flatten().collect()
}
