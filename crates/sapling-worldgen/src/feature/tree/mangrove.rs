//! `mangrove_trunk`: a column whose non-top logs each flip a coin for a branch; every log and
//! branch cell becomes a canopy candidate.

use super::Anchor;
use super::common::{allows, valid_tree_position};
use super::spec::{MangroveTrunk, Tree};
use crate::feature::place::Ctx;
use crate::feature::view::View;
use crate::random::Mt19937;

/// By facing 2..6 (north, south, west, east) less 2.
const BRANCH_STEP: [(i32, i32); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];

impl MangroveTrunk {
    pub fn place(&self, tree: &Tree, ctx: &mut Ctx, origin: [i32; 3]) -> Option<[i32; 3]> {
        let a = ctx.random.next_int(self.height_rand_a + 1);
        let height = self.height_base + a + ctx.random.next_int(self.height_rand_b + 1);
        let origin = tree.roots_then(ctx, origin, height)?;
        let [x, y, z] = origin;
        if y <= crate::terrain::MIN_Y
            || y + height >= crate::terrain::MIN_Y + crate::terrain::HEIGHT as i32
            || !allows(tree.may_grow_on, ctx.view.block(x, y - 1, z))
        {
            return None;
        }
        let mut candidates = Vec::new();
        let mut anchor = origin;
        for i in 0..height {
            let p = [x, y + i, z];
            if valid_tree_position(ctx.view, p, tree.may_replace) && ctx.view.set_block(p[0], p[1], p[2], self.block) {
                if let Some(decoration) = &self.decoration {
                    decoration.place(ctx.view, p, [true; 4], ctx.random);
                }
                // The coin is flipped for every non-top log, "branches" configured or not.
                if i < height - 1 && ctx.random.next_boolean() {
                    let facing = ctx.random.next_int(4) as usize;
                    let first = self.branch_length.draw_inclusive(ctx.random);
                    let second = self.branch_length.draw_inclusive(ctx.random);
                    let steps = self.branch_steps.draw_inclusive(ctx.random);
                    let start = (first - second - 1).max(0);
                    let branch = Branch { tree, facing, start, steps, height };
                    branch.place(self, ctx.view, ctx.random, p, &mut candidates);
                }
            }
            if i == height - 1 {
                anchor = [x, y + i + 1, z];
                candidates.push(anchor);
            }
        }
        tree.place_canopies(ctx, &[Anchor { at: anchor, candidates }]);
        Some(origin)
    }
}

struct Branch<'t> {
    tree: &'t Tree,
    facing: usize,
    start: i32,
    steps: i32,
    height: i32,
}

impl Branch<'_> {
    /// Steps sideways one cell per level from `start` above the log, while levels stay under the
    /// trunk height and `steps` lasts; every cell is a candidate, the tip two under its top too.
    fn place(&self, trunk: &MangroveTrunk, view: &mut View, random: &mut Mt19937, [x0, base, z0]: [i32; 3], candidates: &mut Vec<[i32; 3]>) {
        let (dx, dz) = BRANCH_STEP[self.facing];
        let (mut x, mut z) = (x0, z0);
        let mut tip = self.start + base;
        let (mut last_x, mut last_z) = (x, z);
        if self.start < self.height && self.steps >= 1 {
            let (mut v, mut steps) = (self.start, self.steps);
            loop {
                if v >= 1 {
                    x += dx;
                    z += dz;
                    let p = [x, v + base, z];
                    tip = p[1];
                    if valid_tree_position(view, p, self.tree.may_replace) && view.set_block(p[0], p[1], p[2], trunk.block) {
                        if let Some(decoration) = &trunk.decoration {
                            decoration.place(view, p, [true; 4], random);
                        }
                        tip = p[1] + 1;
                    }
                    (last_x, last_z) = (x, z);
                    candidates.push(p);
                }
                v += 1;
                if v >= self.height {
                    break;
                }
                steps -= 1;
                if steps + 1 <= 1 {
                    break;
                }
            }
        }
        if tip - base > 1 {
            candidates.push([last_x, tip - 2, last_z]);
        }
    }
}
