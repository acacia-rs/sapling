//! `cherry_trunk`: a stem and one or two branches walking out to a drawn tip, each tip (and the
//! stem top for the two-branches-and-trunk type) getting the canopy; and `cherry_canopy`.

use super::Anchor;
use super::common::{allows, valid_tree_position};
use super::spec::{CherryCanopy, CherryTrunk, Range, Tree};
use crate::block::Block;
use crate::feature::place::Ctx;
use crate::feature::view::View;
use crate::random::Mt19937;

/// The acacia lean table, indexed by a `nextInt(4)` direction.
const LEAN_X: [i32; 4] = [0, -1, 0, 1];
const LEAN_Z: [i32; 4] = [1, 0, -1, 0];

fn log(view: &mut View, tree: &Tree, [x, y, z]: [i32; 3], block: Block) {
    if valid_tree_position(view, [x, y, z], tree.may_replace) {
        view.set_block(x, y, z, block);
    }
}

impl CherryTrunk {
    fn height(&self, random: &mut Mt19937) -> i32 {
        self.height_base + self.height_intervals.iter().map(|&i| Range(0, i).draw_inclusive(random)).sum::<i32>()
    }

    /// 0 one branch, 1 two branches, 2 two branches and the trunk; an exhausted scan (-1) acts as 0.
    fn tree_type(&self, random: &mut Mt19937) -> i32 {
        let mut pick = random.next_int(self.weights.iter().sum());
        for (i, &weight) in self.weights.iter().enumerate() {
            if pick < weight {
                return i as i32;
            }
            pick -= weight;
        }
        -1
    }

    /// Walks from beside the stem at `start` to the tip, one vertical or horizontal step per
    /// `nextFloat`; the canopy goes one above the tip.
    #[allow(clippy::too_many_arguments)]
    fn branch(&self, tree: &Tree, view: &mut View, random: &mut Mt19937, [ox, oy, oz]: [i32; 3], height: i32, direction: usize, start: i32, joins_below_top: bool) -> [i32; 3] {
        let end_y = height + self.end_offset.draw_inclusive(random) - 1;
        let extra = joins_below_top || end_y < start;
        let length = self.horizontal_length.draw_inclusive(random) + i32::from(extra);
        let (dx, dz) = (LEAN_X[direction], LEAN_Z[direction]);
        let side = if dx != 0 { self.block_x } else { self.block_z };
        let mut cur = [ox + dx, oy + start, oz + dz];
        log(view, tree, cur, side);
        if extra {
            cur = [cur[0] + dx, cur[1], cur[2] + dz];
            log(view, tree, cur, side);
        }
        let target = [ox + dx * length, oy + end_y, oz + dz * length];
        while cur != target {
            let dy = target[1] - cur[1];
            let distance = (target[0] - cur[0]).abs() + dy.abs() + (target[2] - cur[2]).abs();
            if random.next_float() < dy.abs() as f32 / distance as f32 {
                cur[1] += dy.signum();
                log(view, tree, cur, self.block);
            } else {
                cur = [cur[0] + dx, cur[1], cur[2] + dz];
                log(view, tree, cur, side);
            }
        }
        [target[0], target[1] + 1, target[2]]
    }

    pub fn place(&self, tree: &Tree, ctx: &mut Ctx, origin: [i32; 3]) -> Option<[i32; 3]> {
        let height = self.height(ctx.random);
        let origin = tree.roots_then(ctx, origin, height)?;
        if !tree.prepare(ctx.view, origin, height, 1) {
            return None;
        }
        let [x, y, z] = origin;
        tree.place_base_block(ctx.view, [x, y - 1, z]);
        let random = &mut *ctx.random;
        let tree_type = self.tree_type(random);
        let mut start_a = height - 1 + self.start_offset.draw_inclusive(random);
        let mut start_b = (height - 1 + Range(self.start_offset.0, self.start_offset.1 - 1).draw_inclusive(random)).max(0);
        if start_a <= start_b {
            start_b += 1;
        }
        let direction = random.next_int(4) as usize;
        if std::env::var_os("TRACE_TREES").is_some() {
            eprintln!("cherry at {origin:?} height {height} type {tree_type} a {start_a} b {start_b} dir {direction} drawn {}", random.drawn);
        }
        start_a = start_a.max(0);
        let stems = match tree_type {
            1 => start_a.max(start_b) + 1,
            2 => height,
            _ => start_a + 1,
        };
        for i in 0..stems {
            log(ctx.view, tree, [x, y + i, z], self.block);
        }
        let mut anchors = Vec::with_capacity(3);
        if tree_type == 2 {
            anchors.push(Anchor::at([x, y + height, z]));
        }
        anchors.push(Anchor::at(self.branch(tree, ctx.view, ctx.random, origin, height, direction, start_a, start_a < stems - 1)));
        if tree_type >= 1 {
            let tip = self.branch(tree, ctx.view, ctx.random, origin, height, (direction + 2) & 3, start_b, start_b < stems - 1);
            anchors.push(Anchor::at(tip));
        }
        tree.place_canopies(ctx, &anchors);
        Some(origin)
    }
}

fn distance_to_trunk(offset: i32, width: i32) -> i32 {
    if width < 2 { offset.abs() } else { offset.abs().min((offset - (width - 1)).abs()) }
}

/// Edge walks of the hanging leaves: counter-clockwise neighbour of each direction, and steps.
const COUNTER_CLOCKWISE: [usize; 4] = [3, 0, 1, 2];

impl CherryCanopy {
    /// Chance rolls come before the `may_replace` check, so rejected cells still draw.
    fn layer(&self, tree: &Tree, view: &mut View, random: &mut Mt19937, [x, y, z]: [i32; 3], dy: i32, radius: i32) {
        for dx in -radius..self.trunk_width + radius {
            let x_dist = distance_to_trunk(dx, self.trunk_width);
            for dz in -radius..self.trunk_width + radius {
                let z_dist = distance_to_trunk(dz, self.trunk_width);
                if dy == -1 && (x_dist == radius || z_dist == radius) && self.wide_bottom_hole.roll(random) {
                    continue;
                }
                let corner = x_dist == radius && z_dist == radius;
                if radius < 3 {
                    if corner && self.corner_hole.roll(random) {
                        continue;
                    }
                } else if corner || x_dist + z_dist > 2 * radius - 2 && self.corner_hole.roll(random) {
                    continue;
                }
                let p = [x + dx, y + dy, z + dz];
                if allows(tree.may_replace, view.block(p[0], p[1], p[2])) {
                    view.set_block(p[0], p[1], p[2], self.leaf);
                }
            }
        }
    }

    /// The layer, then along its outer ring's edges a chance of a leaf below each leaf, and after
    /// a placed one a chance of a second below it.
    fn layer_with_hanging(&self, tree: &Tree, view: &mut View, random: &mut Mt19937, at: [i32; 3], dy: i32, radius: i32) {
        self.layer(tree, view, random, at, dy, radius);
        let count = self.trunk_width + 2 * radius - 1;
        if count <= 0 {
            return;
        }
        let y = at[1] + dy;
        for direction in 0..4 {
            let ccw = COUNTER_CLOCKWISE[direction];
            let edge = if ccw == 3 || ccw == 0 { self.trunk_width + radius - 1 } else { radius };
            let mut x = at[0] + edge * LEAN_X[ccw] - radius * LEAN_X[direction];
            let mut z = at[2] + edge * LEAN_Z[ccw] - radius * LEAN_Z[direction];
            for _ in 0..count {
                if view.block(x, y, z) == self.leaf
                    && self.hanging_leaves.roll(random)
                    && allows(tree.may_replace, view.block(x, y - 1, z))
                    && view.set_block(x, y - 1, z, self.leaf)
                    && self.hanging_leaves_extension.roll(random)
                    && allows(tree.may_replace, view.block(x, y - 2, z))
                {
                    view.set_block(x, y - 2, z, self.leaf);
                }
                x += LEAN_X[direction];
                z += LEAN_Z[direction];
            }
        }
    }

    pub fn place(&self, tree: &Tree, view: &mut View, random: &mut Mt19937, anchor: &Anchor) {
        let at = anchor.at;
        let radius = self.radius.draw_inclusive(random);
        let height = self.height.draw_inclusive(random);
        let (a, b, c) = (self.trunk_width + radius - 4, self.trunk_width + radius - 3, self.trunk_width + radius - 2);
        self.layer(tree, view, random, at, height - 3, a);
        self.layer(tree, view, random, at, height - 4, b);
        for dy in (0..=height - 5).rev() {
            self.layer(tree, view, random, at, dy, c);
        }
        self.layer_with_hanging(tree, view, random, at, -1, c);
        self.layer_with_hanging(tree, view, random, at, -2, b);
    }
}
