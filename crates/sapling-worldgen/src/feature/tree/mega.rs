//! `mega_trunk` (giant spruces and pines, mega jungle trees): the top canopy first, then the
//! branches with their own canopies, the trunk column, the base block and the `base_cluster`.

use super::Anchor;
use super::common::{allows, valid_tree_position};
use super::spec::{BaseCluster, MegaBranches, MegaTrunk, Tree};
use crate::block::Block;
use crate::feature::place::Ctx;
use crate::feature::view::View;
use crate::random::Mt19937;

pub use super::mega_canopy::canopy;

impl MegaTrunk {
    pub fn place(&self, tree: &Tree, ctx: &mut Ctx, origin: [i32; 3]) -> Option<[i32; 3]> {
        let height = self.height_intervals.iter().fold(self.height_base, |h, &i| h + ctx.random.next_int(i));
        let origin = tree.roots_then(ctx, origin, height)?;
        if height < 1 || !tree.prepare(ctx.view, origin, height, self.width) {
            return None;
        }
        let [x, y, z] = origin;
        // The top canopy is placed before any branch or log, above the origin column.
        tree.place_canopies(ctx, &[Anchor::at([x, y + height, z])]);
        if let Some(branches) = &self.branches {
            branches.place(tree, ctx.view, ctx.random, origin, height, self.block);
        }
        let width = self.width;
        for dx in 0..width {
            for dz in 0..width {
                if valid_tree_position(ctx.view, [x + dx, y, z + dz], tree.may_replace) {
                    ctx.view.set_block(x + dx, y, z + dz, self.block);
                }
            }
        }
        for dy in 1..height - 1 {
            for dx in 0..width {
                for dz in 0..width {
                    let p = [x + dx, y + dy, z + dz];
                    if !valid_tree_position(ctx.view, p, tree.may_replace) {
                        continue;
                    }
                    ctx.view.set_block(p[0], p[1], p[2], self.block);
                    if let Some(decoration) = &self.decoration {
                        // West, east, north, south; east/south only when not also the west/north edge.
                        let mask = [dx == 0, dx != 0 && dx == width - 1, dz == 0, dz != 0 && dz == width - 1];
                        decoration.place(ctx.view, p, mask, ctx.random);
                    }
                }
            }
        }
        if valid_tree_position(ctx.view, [x, y + height - 1, z], tree.may_replace) {
            ctx.view.set_block(x, y + height - 1, z, self.block);
        }
        for dx in 0..width {
            for dz in 0..width {
                tree.place_base_block(ctx.view, [x + dx, y - 1, z + dz]);
            }
        }
        if let (Some(cluster), Some(&target)) = (&tree.base_cluster, tree.base_block.first()) {
            cluster.place(ctx.view, ctx.random, [x, y - 1, z], width, target);
        }
        Some(origin)
    }
}

impl MegaBranches {
    /// From `altitude_factor.1` of the height down to `.0`, a branch every `interval`, each a
    /// sloped line of logs at a random angle (Mth table trig) with its canopy at the tip.
    fn place(&self, tree: &Tree, view: &mut View, random: &mut Mt19937, [x, y, z]: [i32; 3], height: i32, block: Block) {
        if self.length < 1 {
            return;
        }
        let top = (height as f32 * self.altitude_factor.1) as i32 + y;
        let low = (height as f32 * self.altitude_factor.0) as i32 + y;
        let mut level = top - self.interval.draw(random);
        while level > low {
            let angle = random.next_float() * std::f32::consts::PI * 2.0;
            let (cos, sin) = (crate::mth::cos(angle), crate::mth::sin(angle));
            let start = ((level - 1) as f32 - self.slope * (self.length - 1) as f32) as i32;
            let (mut tip_x, mut tip_z) = (x, z);
            for step in 0..self.length {
                let bx = x + (1.5 + cos * step as f32) as i32;
                let bz = z + (1.5 + sin * step as f32) as i32;
                let by = (start as f32 + self.slope * step as f32) as i32;
                view.set_block(bx, by, bz, block);
                (tip_x, tip_z) = (bx, bz);
            }
            if let Some(canopy) = self.canopy {
                canopy.place(tree, view, random, &Anchor::at([tip_x, level, tip_z]));
            }
            level -= self.interval.draw(random);
        }
    }
}

impl BaseCluster {
    /// Circles of `target` at the four corners around the footprint, then `count` draws of an 8x8
    /// window around `ground` whose border cells get a circle too.
    fn place(&self, view: &mut View, random: &mut Mt19937, [x, y, z]: [i32; 3], reach: i32, target: Block) {
        if self.count <= 0 {
            return;
        }
        for (dx, dz) in [(-1, -1), (reach, -1), (-1, reach), (reach, reach)] {
            self.circle(view, [x + dx, y, z + dz], target);
        }
        for _ in 0..self.count {
            let r = random.next_int(64);
            let (qx, rem) = (r / 8, r % 8);
            if qx == 0 || qx == 7 || rem == 0 || rem == 7 {
                self.circle(view, [x + rem - 3, y, z + qx - 3], target);
            }
        }
    }

    /// A square of `radius` without its corners; per column the first of y+2..=y-3 that already
    /// is `target` (stop) or passes `may_replace` (replaced).
    fn circle(&self, view: &mut View, [x, y, z]: [i32; 3], target: Block) {
        let radius = self.radius;
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                if dx.abs() == radius && dz.abs() == radius {
                    continue;
                }
                for dy in [2, 1, 0, -1, -2, -3] {
                    let existing = view.block(x + dx, y + dy, z + dz);
                    if existing == target {
                        break;
                    }
                    if allows(self.may_replace, existing) {
                        view.set_block(x + dx, y + dy, z + dz, target);
                        break;
                    }
                }
            }
        }
    }
}
