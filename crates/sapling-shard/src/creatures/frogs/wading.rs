//! A frog's way to land through water, cell by cell, as an amphibious path: across and cornerwise over
//! cells with water in or under them or a floor under them, up or down only straight unless onto a
//! floor (as Java's `AmphibiousNodeEvaluator` adds its water nodes), a cell of open water costing
//! [`OPEN_WATER`] more, as Java's frog weighs water (`PathType.WATER` malus 4), so the way keeps to the
//! pool floor and rises where it must (bds-f14efish.txt: along the floor, then up beside the wall).

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

use sapling_world::World as Level;

use crate::creatures::harm;

type Cell = [i32; 3];

const WATER: [&str; 2] = ["minecraft:water", "minecraft:flowing_water"];
/// A step costs ten times its length, a cell of open water this many steps more.
const STEP: f32 = 10.0;
const OPEN_WATER: u32 = 40;
/// A search looks at no more cells than this.
const MOST: usize = 6000;

fn holds(level: &Level, [x, y, z]: Cell) -> bool {
    level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| !state.boxes.is_empty())
}

fn is_water(level: &Level, cell: Cell) -> bool {
    harm::holds(level, cell, &WATER)
}

/// Whether a frog may be in `cell`: nothing solid in it, and water in it or under it, or a floor under it.
fn passable(level: &Level, [x, y, z]: Cell) -> bool {
    let below = [x, y - 1, z];
    level.in_height(y) && !holds(level, [x, y, z]) && (is_water(level, [x, y, z]) || is_water(level, below) || holds(level, below))
}

/// What stepping into `cell` costs beyond its length: a cell of or over water with no floor under it is open water.
fn malus(level: &Level, [x, y, z]: Cell) -> u32 {
    let floating = is_water(level, [x, y, z]) || is_water(level, [x, y - 1, z]);
    if floating && !holds(level, [x, y - 1, z]) { OPEN_WATER } else { 0 }
}

/// The cells of the cheapest way from `from` to `to`, after `from`; `None` where none is found.
pub(super) fn route(level: &Level, from: Cell, to: Cell) -> Option<Vec<Cell>> {
    let length = |a: Cell, b: Cell| ((0..3).map(|axis| ((a[axis] - b[axis]) as f32).powi(2)).sum::<f32>().sqrt() * STEP) as u32;
    let (mut open, mut came, mut cost) = (BinaryHeap::from([(Reverse(length(from, to)), from)]), HashMap::new(), HashMap::from([(from, 0u32)]));
    while let Some((_, cell)) = open.pop() {
        if cell == to {
            let mut way = vec![cell];
            while let Some(before) = came.get(way.last()?).filter(|before| **before != from) {
                way.push(*before);
            }
            way.reverse();
            return Some(way);
        }
        if cost.len() > MOST {
            return None;
        }
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let next = [cell[0] + dx, cell[1] + dy, cell[2] + dz];
                    let slanted = dy != 0 && (dx != 0 || dz != 0) && !holds(level, [next[0], next[1] - 1, next[2]]);
                    if next == cell || slanted || !passable(level, next) {
                        continue;
                    }
                    // No corner is cut: each cell it passes beside on the way is free too.
                    let beside = [[cell[0] + dx, cell[1], cell[2]], [cell[0], cell[1] + dy, cell[2]], [cell[0], cell[1], cell[2] + dz]];
                    if beside.iter().any(|by| *by != cell && holds(level, *by)) {
                        continue;
                    }
                    let reached = cost[&cell] + length(cell, next) + malus(level, next);
                    if cost.get(&next).is_none_or(|known| reached < *known) {
                        cost.insert(next, reached);
                        came.insert(next, cell);
                        open.push((Reverse(reached + length(next, to)), next));
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frogs_way_keeps_to_the_pool_floor_and_rises_by_the_wall() {
        let mut level = Level::void();
        let block = |level: &Level, name: &str| level.block_id(name).unwrap();
        let (stone, water) = (block(&level, "minecraft:stone"), block(&level, "minecraft:water"));
        level.fill([-7, 60, -7], [7, 66, 7], stone);
        level.fill([-6, 61, -6], [6, 66, 6], water);
        let way = route(&level, [0, 61, 0], [7, 67, 0]).expect("a way out");
        assert_eq!(way.last(), Some(&[7, 67, 0]));
        let off_floor = way.iter().filter(|cell| cell[1] > 61 && cell[0] < 6).count();
        assert!(off_floor <= 1, "along the floor, up by the wall: {way:?}");
    }
}
