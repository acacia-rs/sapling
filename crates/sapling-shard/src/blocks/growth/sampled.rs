//! The cells of what lies thick in caves and seas (lava, vines, cave vines, kelp, mushrooms), whose
//! random ticks are drawn rather than rolled cell by cell: a tick picks as many of a world's cells
//! as the rule's chance gives them all, so each has that chance on average and a cave full of them
//! costs a tick only what it changes.

use std::collections::{HashMap, HashSet};

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::Picks;

#[derive(Default)]
struct Cells {
    list: Vec<BlockPos>,
    known: HashSet<BlockPos>,
    /// The part of a pick the last ticks' chances left over.
    owed: f32,
}

#[derive(Resource, Default)]
pub(crate) struct Sampled(HashMap<Entity, Cells>);

/// Keeps track of `cells` in `layer`.
pub(super) fn keep(world: &mut World, layer: Entity, cells: impl IntoIterator<Item = BlockPos>) {
    let mut sampled = world.get_resource_or_insert_with(Sampled::default);
    let kept = sampled.0.entry(layer).or_default();
    for pos in cells {
        if kept.known.insert(pos) {
            kept.list.push(pos);
        }
    }
}

/// The cells of `layer` whose random tick comes up this tick, each having `chance`; a cell is
/// picked at most once a tick.
pub(super) fn picked(world: &mut World, layer: Entity, chance: f32, picks: &mut Picks) -> Vec<BlockPos> {
    let Some(kept) = world.get_resource_mut::<Sampled>().map(|sampled| sampled.into_inner()).and_then(|sampled| sampled.0.get_mut(&layer)) else { return Vec::new() };
    if kept.list.is_empty() {
        return Vec::new();
    }
    let due = kept.list.len() as f32 * chance + kept.owed;
    let count = due.floor();
    kept.owed = due - count;
    let mut chosen: Vec<BlockPos> = (0..count as usize).map(|_| kept.list[picks.below(kept.list.len() as u32) as usize]).collect();
    chosen.sort_unstable();
    chosen.dedup();
    chosen
}

/// Stops keeping track of `pos` in `layer`: what was there is gone.
pub(super) fn forget(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(mut sampled) = world.get_resource_mut::<Sampled>() else { return };
    let Some(kept) = sampled.0.get_mut(&layer) else { return };
    if kept.known.remove(&pos)
        && let Some(index) = kept.list.iter().position(|at| *at == pos)
    {
        kept.list.swap_remove(index);
    }
}

/// Every layer kept track of.
pub(super) fn layers(world: &World) -> Vec<Entity> {
    world.get_resource::<Sampled>().map(|sampled| sampled.0.keys().copied().collect()).unwrap_or_default()
}
