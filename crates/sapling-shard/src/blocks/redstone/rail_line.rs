//! A line of powered or activator rails: a rail that gets a signal passes it eight rails on each
//! way along its kind, up and down slopes (the Java game's `PoweredRailBlock.findPoweredRailSignal`;
//! research/captures/bds-rails.txt: a block of redstone beside the first of twelve lit nine).

use std::collections::{HashSet, VecDeque};

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::power_at;
use crate::blocks::{level, send, with_property};

/// Rails a signal runs past the one that got it.
const REACH: u8 = 8;
const NS: u8 = 0;
const EW: u8 = 1;
/// Rails looked at for one change: two reaches and the slopes' second looks.
const MAX_LOOKED: usize = 64;

/// A rail's kind, shape and whether it is on.
fn rail(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Option<(&'static str, u8, bool)> {
    let level = level(world, layer).filter(|level| level.in_height(y) && level.is_loaded(send::column([x, y, z])))?;
    let state = level.registry().get(level.block(x, y, z)).filter(|state| matches!(state.name, "minecraft:golden_rail" | "minecraft:activator_rail"))?;
    let shape = state.property("rail_direction")?.parse().ok()?;
    Some((state.name, shape, state.property("rail_data_bit") == Some("1")))
}

/// The next cell along a rail of `shape` one way or the other, and whether a rail one lower
/// there joins too (not where the way climbs); the straight shape a joining rail must not cross.
fn next([x, y, z]: BlockPos, shape: u8, forward: bool) -> (BlockPos, bool, u8) {
    match (shape, forward) {
        (NS, true) => ([x, y, z + 1], true, NS),
        (NS, false) => ([x, y, z - 1], true, NS),
        (2, true) | (EW, true) => ([x - 1, y, z], true, EW),
        (2, false) => ([x + 1, y + 1, z], false, EW),
        (3, true) => ([x - 1, y + 1, z], false, EW),
        (3, false) | (EW, false) => ([x + 1, y, z], true, EW),
        (4, true) | (5, false) => ([x, y, z + 1 - 2 * i32::from(shape == 5)], true, NS),
        (4, false) => ([x, y + 1, z - 1], false, NS),
        (5, true) => ([x, y + 1, z + 1], false, NS),
        _ => ([x, y, z], false, shape),
    }
}

/// Whether a rail of `shape` runs along `line` (north-south or east-west, slopes included).
fn runs_along(shape: u8, line: u8) -> bool {
    match line {
        NS => matches!(shape, NS | 4 | 5),
        _ => matches!(shape, EW | 2 | 3),
    }
}

/// The rails of `kind` joined to the one at `pos` one way, with their shapes.
fn joined(world: &World, layer: Entity, pos: BlockPos, shape: u8, forward: bool, kind: &str) -> Vec<(BlockPos, u8, bool)> {
    let (cell, lower_too, line) = next(pos, shape, forward);
    let below = [cell[0], cell[1] - 1, cell[2]];
    [Some(cell), lower_too.then_some(below)]
        .into_iter()
        .flatten()
        .filter_map(|at| rail(world, layer, at).filter(|(name, shape, _)| *name == kind && runs_along(*shape, line)).map(|(_, shape, on)| (at, shape, on)))
        .collect()
}

/// Whether a rail on and powered itself is within reach of `pos` one way, through rails that are on.
fn fed(world: &World, layer: Entity, pos: BlockPos, shape: u8, forward: bool, kind: &str, depth: u8) -> bool {
    depth < REACH
        && joined(world, layer, pos, shape, forward, kind).into_iter().any(|(at, shape, on)| on && (power_at(world, layer, at) > 0 || fed(world, layer, at, shape, forward, kind, depth + 1)))
}

/// Turns the rail at `pos`, which got a new signal, and the rails of its line to what reaches
/// them, nearest first.
pub(super) fn relight(world: &mut World, layer: Entity, pos: BlockPos) {
    let (mut waiting, mut looked) = (VecDeque::from([pos]), HashSet::new());
    while let Some(at) = waiting.pop_front() {
        if looked.len() >= MAX_LOOKED || !looked.insert(at) {
            continue;
        }
        let Some((kind, shape, on)) = rail(world, layer, at) else { continue };
        let now = power_at(world, layer, at) > 0 || fed(world, layer, at, shape, true, kind, 0) || fed(world, layer, at, shape, false, kind, 0);
        if now != on
            && let Some(level) = level(world, layer)
            && let Some(id) = with_property(level.registry(), level.block(at[0], at[1], at[2]), "rail_data_bit", if now { "1" } else { "0" })
        {
            send::set(world, layer, at, id);
        }
        for forward in [true, false] {
            waiting.extend(joined(world, layer, at, shape, forward, kind).into_iter().map(|(next, ..)| next));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slope_leads_up_one_way_and_on_or_down_the_other() {
        assert_eq!(next([0, 0, 0], 2, false), ([1, 1, 0], false, EW));
        assert_eq!(next([0, 0, 0], 2, true), ([-1, 0, 0], true, EW));
        assert_eq!(next([0, 0, 0], 5, true), ([0, 1, 1], false, NS));
        assert_eq!(next([0, 0, 0], 5, false), ([0, 0, -1], true, NS));
        assert_eq!(next([0, 0, 0], 4, true), ([0, 0, 1], true, NS));
        assert!(runs_along(2, EW) && !runs_along(4, EW));
    }
}
