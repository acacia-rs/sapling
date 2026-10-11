//! A way around what stands between a mob and where it goes, cell by cell
//! (docs/research/mob-attacks.md, "Around a fence"): BDS's zombie went to the cell past a
//! fence's end and on from there.
// TODO: the shortest way over whole cells, not the game's own search: doors, water, harm on the
// way and the mob's width are not weighed, and a way longer than `REACH` cells is not found.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

use sapling_world::World as Level;

/// A search looks no further than this many cells from where it starts, and at no more cells
/// than this.
const REACH: i32 = 24;
const MOST: usize = 1500;
/// A mob drops this many cells onto a lower floor and no further.
const DROP: i32 = 3;
/// A step across costs this, a step cornerwise that.
const ACROSS: u32 = 10;
const CORNERWISE: u32 = 14;

type Cell = [i32; 3];

fn holds(level: &Level, [x, y, z]: Cell) -> bool {
    level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| !state.boxes.is_empty())
}

/// Whether a mob two cells high can stand in the cell: a whole block under it, nothing in it.
pub(super) fn is_standable(level: &Level, [x, y, z]: Cell) -> bool {
    let floor = level.in_height(y - 1) && level.registry().get(level.block(x, y - 1, z)).is_some_and(|state| state.is_full_cube());
    floor && level.is_loaded(sapling_world::ChunkPos::of_block(x, z)) && !holds(level, [x, y, z]) && !holds(level, [x, y + 1, z])
}

/// The cell a mob at the column of `[x, _, z]` coming from the height `y` stands in: level, a
/// step up, or a drop.
fn landing(level: &Level, [x, y, z]: Cell) -> Option<Cell> {
    std::iter::once(y).chain([y + 1]).chain((1..=DROP).map(|down| y - down)).map(|y| [x, y, z]).find(|cell| is_standable(level, *cell))
}

/// Whether a mob can walk straight from `from` to `to` on the floor it stands on.
pub(super) fn is_clear(level: &Level, from: [f32; 3], to: [f32; 3]) -> bool {
    let (dx, dz) = (to[0] - from[0], to[2] - from[2]);
    let steps = (dx.hypot(dz) / 0.5).ceil().max(1.0) as i32;
    let y = from[1].floor() as i32;
    (to[1] - from[1]).abs() < 0.5 && (0..=steps).all(|step| is_standable(level, [(from[0] + dx * step as f32 / steps as f32).floor() as i32, y, (from[2] + dz * step as f32 / steps as f32).floor() as i32]))
}

/// The cells of the shortest way from `from` to `to`, after `from`; `None` where there is none
/// within reach.
pub(super) fn route(level: &Level, from: Cell, to: Cell) -> Option<Vec<Cell>> {
    let ahead = |[x, _, z]: Cell| ((x - to[0]).abs().max((z - to[2]).abs()) as u32) * ACROSS;
    let (mut open, mut came, mut cost) = (BinaryHeap::from([(Reverse(ahead(from)), from)]), HashMap::new(), HashMap::from([(from, 0u32)]));
    while let Some((_, cell)) = open.pop() {
        if [cell[0], cell[2]] == [to[0], to[2]] && (cell[1] - to[1]).abs() <= 1 {
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
        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
            let Some(next) = landing(level, [cell[0] + dx, cell[1], cell[2] + dz]).filter(|next| (next[0] - from[0]).abs() <= REACH && (next[2] - from[2]).abs() <= REACH) else { continue };
            // Cornerwise only on one floor and past two free cells: no corner is cut.
            let cornerwise = dx != 0 && dz != 0;
            if cornerwise && (next[1] != cell[1] || !is_standable(level, [cell[0] + dx, cell[1], cell[2]]) || !is_standable(level, [cell[0], cell[1], cell[2] + dz])) {
                continue;
            }
            let reached = cost[&cell] + if cornerwise { CORNERWISE } else { ACROSS };
            if cost.get(&next).is_none_or(|known| reached < *known) {
                cost.insert(next, reached);
                came.insert(next, cell);
                open.push((Reverse(reached + ahead(next)), next));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_way_goes_round_a_fence_and_over_a_step_and_not_through_a_wall() {
        let mut level = Level::void();
        let block = |level: &Level, name: &str| level.block_id(name).unwrap();
        let (stone, fence) = (block(&level, "minecraft:stone"), block(&level, "minecraft:oak_fence"));
        level.fill([-8, 64, -8], [8, 64, 8], stone);
        (-2..=2).for_each(|x| drop(level.set_block(x, 65, 2, fence)));
        let way = route(&level, [0, 65, 4], [0, 65, 0]).expect("round the fence");
        assert!(way.contains(&[3, 65, 2]) || way.contains(&[-3, 65, 2]), "past its end: {way:?}");
        assert!(!is_clear(&level, [0.5, 65.0, 4.5], [0.5, 65.0, 0.5]) && is_clear(&level, [0.5, 65.0, 4.5], [5.5, 65.0, 4.5]));

        (-8..=8).for_each(|x| drop(level.set_block(x, 65, 2, stone)));
        assert!(route(&level, [0, 65, 4], [0, 65, 0]).is_some_and(|way| way.contains(&[0, 66, 2])), "over a step");
        (-8..=8).for_each(|x| drop(level.set_block(x, 66, 2, stone)));
        assert_eq!(route(&level, [0, 65, 4], [0, 65, 0]), None, "a wall two high across the floor");
    }
}
