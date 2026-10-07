//! `mangrove_roots`: before the trunk, a root column of `y_offset` cells and four recursive root
//! branches (north, east, south, west); the trunk then starts on top of the column.

use super::common::valid_tree_position;
use super::spec::{Decoration, MangroveRoots, Tree};
use crate::block::Block;
use crate::feature::place::Ctx;
use crate::feature::view::View;
use crate::random::Mt19937;

/// North, east, south, west: BDS's order, not shuffled.
const DIRECTIONS: [[i32; 3]; 4] = [[0, 0, -1], [1, 0, 0], [0, 0, 1], [-1, 0, 0]];

fn add([x, y, z]: [i32; 3], [dx, dy, dz]: [i32; 3]) -> [i32; 3] {
    [x + dx, y + dy, z + dz]
}

impl MangroveRoots {
    pub fn place(&self, tree: &Tree, ctx: &mut Ctx, origin: [i32; 3], height: i32) -> Option<[i32; 3]> {
        let [x, y, z] = origin;
        let value = self.y_offset.draw(ctx.random);
        let top = y + value;
        // MangroveTreeRoots::placeRoots (1438f0d00) runs prepareSpawn over the roots and the trunk together.
        if !tree.prepare(ctx.view, origin, height + value, 1) {
            return None;
        }
        if (y..top).any(|cy| !self.can_place(ctx.view, [x, cy, z])) {
            return None;
        }
        let base = [x, top, z];
        let mut roots = vec![[x, top - 1, z]];
        for dir in DIRECTIONS {
            let start = add(base, dir);
            let mut branch = Vec::new();
            if !self.simulate(ctx.view, ctx.random, start, base, dir, &mut branch, 0) || branch.is_empty() {
                return None;
            }
            roots.append(&mut branch);
            roots.push(start);
        }
        for p in roots {
            self.place_root(ctx.view, ctx.random, p);
        }
        Some(base)
    }

    /// `roots_may_grow_through`, or air, water, leaves or a plant a tree grows through (1438f16f0).
    fn can_place(&self, view: &View, at: [i32; 3]) -> bool {
        valid_tree_position(view, at, self.may_grow_through)
    }

    /// Down only beyond `max_width`; within `max_width - 3` a float (< 0.2: down) then a boolean
    /// (forward or down); in between always down and, on a float < 0.2, also forward-and-down.
    fn next_positions(&self, random: &mut Mt19937, at: [i32; 3], origin: [i32; 3], dir: [i32; 3]) -> Vec<[i32; 3]> {
        let distance = (0..3).map(|i| (at[i] - origin[i]).abs()).sum::<i32>();
        let down = add(at, [0, -1, 0]);
        let forward = add(at, dir);
        if self.max_width < distance {
            return vec![down];
        }
        if self.max_width - 3 >= distance {
            if random.next_float() >= 0.2 && random.next_boolean() {
                return vec![forward];
            }
            return vec![down];
        }
        if random.next_float() < 0.2 { vec![down, add(forward, [0, -1, 0])] } else { vec![down] }
    }

    /// Any failure (too long, at any depth) empties the whole branch.
    #[allow(clippy::too_many_arguments)]
    fn simulate(&self, view: &View, random: &mut Mt19937, at: [i32; 3], origin: [i32; 3], dir: [i32; 3], branch: &mut Vec<[i32; 3]>, depth: i32) -> bool {
        if depth == self.max_length || branch.len() as i32 > self.max_length {
            branch.clear();
            return false;
        }
        for next in self.next_positions(random, at, origin, dir) {
            if !self.can_place(view, next) {
                continue;
            }
            branch.push(next);
            if !self.simulate(view, random, next, origin, dir, branch, depth + 1) {
                branch.clear();
                return false;
            }
        }
        true
    }

    /// Mud becomes muddy roots; anything else roots, then maybe the above-root block on air
    /// (featurelab approximates BDS's gate there as air).
    fn place_root(&self, view: &mut View, random: &mut Mt19937, at: [i32; 3]) {
        if !self.can_place(view, at) {
            return;
        }
        let decoration = self.decoration.as_ref();
        if view.block(at[0], at[1], at[2]) == self.mud {
            Decoration::place_decorated(decoration, view, at, self.muddy_root, random);
            return;
        }
        Decoration::place_decorated(decoration, view, at, self.root, random);
        let Some((block, odds)) = self.above_root else { return };
        if !odds.is_valid() || !odds.roll(random) {
            return;
        }
        let above = add(at, [0, 1, 0]);
        if view.block(above[0], above[1], above[2]) == Block::Air {
            Decoration::place_decorated(decoration, view, above, block, random);
        }
    }
}
