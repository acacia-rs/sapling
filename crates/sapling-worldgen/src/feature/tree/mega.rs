//! `MegaTreeTrunk::placeTrunk` (1438f2ba0; height 1438f4080): its own spawn check, the top canopy,
//! the branches with their canopies, the trunk column, the base block and the `base_cluster`.

use super::Anchor;
use super::common::allows;
use super::spec::{BaseCluster, MegaBranches, MegaTrunk, Tree};
use crate::block::Block;
use crate::feature::place::Ctx;
use crate::feature::view::View;
use crate::random::Mt19937;
use crate::terrain::{HEIGHT, MIN_Y};

pub use super::mega_canopy::canopy;

const TOP: i32 = MIN_Y + HEIGHT as i32;

impl MegaTrunk {
    pub fn place(&self, tree: &Tree, ctx: &mut Ctx, origin: [i32; 3]) -> Option<[i32; 3]> {
        let height = self.height_intervals.iter().fold(self.height_base, |h, &i| h + ctx.random.next_int(i));
        let origin = tree.roots_then(ctx, origin, height)?;
        if !self.spawn(tree, ctx.view, origin, height) {
            return None;
        }
        let [x, y, z] = origin;
        let width = self.width;
        tree.place_canopies(ctx, &[Anchor::at([x, y + height, z])]);
        if let Some(branches) = &self.branches {
            branches.place(tree, ctx.view, ctx.random, origin, height, self.block);
        }
        let replaceable = |view: &View, [x, y, z]: [i32; 3]| allows(tree.may_replace, view.block(x, y, z));
        for dx in 0..width {
            for dz in 0..width {
                if replaceable(ctx.view, [x + dx, y, z + dz]) {
                    ctx.view.set_block(x + dx, y, z + dz, self.block);
                }
            }
        }
        for dy in 1..height - 1 {
            for dx in 0..width {
                for dz in 0..width {
                    let p = [x + dx, y + dy, z + dz];
                    if !replaceable(ctx.view, p) || !ctx.view.set_block(p[0], p[1], p[2], self.block) {
                        continue;
                    }
                    if let Some(decoration) = &self.decoration {
                        // West, east, north, south; east/south only when not also the west/north edge.
                        let mask = [dx == 0, dx != 0 && dx == width - 1, dz == 0, dz != 0 && dz == width - 1];
                        decoration.place(ctx.view, p, mask, ctx.random);
                    }
                }
            }
        }
        if replaceable(ctx.view, [x, y + height - 1, z]) {
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

    /// In the world, `may_grow_on` under the origin cell only, and a volume around the origin column
    /// (radius `max(|width - 1|, 1)` at the base, `width` above, up to one over the top) growable through.
    fn spawn(&self, tree: &Tree, view: &View, [x, y, z]: [i32; 3], height: i32) -> bool {
        if y <= MIN_Y || y + height >= TOP || !allows(tree.may_grow_on, view.block(x, y - 1, z)) {
            return false;
        }
        (0..height + 2).all(|dy| {
            let r = if dy == 0 { (self.width - 1).abs().max(1) } else { self.width };
            (MIN_Y..TOP).contains(&(y + dy))
                && (x - r..=x + r).all(|cx| (z - r..=z + r).all(|cz| tree.grows_through(view.block(cx, y + dy, cz))))
        })
    }
}

impl MegaBranches {
    /// From `altitude_factor.1` of the height (less an interval) down to `.0`, a branch every
    /// `interval`: a sloped line of logs at a random angle (Mth table) with its canopy at the tip.
    fn place(&self, tree: &Tree, view: &mut View, random: &mut Mt19937, [x, y, z]: [i32; 3], height: i32, block: Block) {
        if self.length < 1 {
            return;
        }
        let h = height as f32;
        let mut level = ((y - self.interval.draw(random)) as f32 + self.altitude_factor.1 * h) as i32;
        while self.altitude_factor.0 * h + (y as f32) < level as f32 {
            let angle = random.next_float() * std::f32::consts::PI * 2.0;
            let (cos, sin) = (crate::mth::cos(angle), crate::mth::sin(angle));
            let start = (level - 1) as f32 - (self.length - 1) as f32 * self.slope;
            let (mut tip_x, mut tip_z) = (x, z);
            for step in 0..self.length {
                let s = step as f32;
                tip_x = (cos * s + 1.5 + x as f32) as i32;
                tip_z = (sin * s + 1.5 + z as f32) as i32;
                view.set_block(tip_x, (s * self.slope + start) as i32, tip_z, block);
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
