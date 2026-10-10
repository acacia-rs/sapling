//! `poplar_trunk` (`placeTrunk` 1438f4e80) and `poplar_canopy` (`placeCanopy` 1438f43a0, layer
//! 1438f4c10), read off the decompile: no oracle world grows a poplar, so the port is draw for
//! draw by construction and unverified (docs/research/worldgen-bds-trees.md, "Poplars").

use super::common::allows;
use super::spec::{PoplarCanopy, PoplarTrunk, Tree};
use super::Anchor;
use crate::feature::data::FEATURES;
use crate::feature::place::Ctx;
use crate::feature::view::View;
use crate::random::Mt19937;
use crate::terrain::{HEIGHT, MIN_Y};

/// The branch sides before their shuffle: north, east, south, west.
const SIDES: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

impl PoplarTrunk {
    pub fn place(&self, tree: &Tree, ctx: &mut Ctx, origin: [i32; 3]) -> Option<[i32; 3]> {
        let height = self.height.draw(ctx.random);
        let origin = tree.roots_then(ctx, origin, height)?;
        let [x, y, z] = origin;
        if !tree.prepare(ctx.view, origin, height, 1) || y >= MIN_Y + HEIGHT as i32 - height - 1 {
            return None;
        }
        tree.place_base_block(ctx.view, [x, y - 1, z]);
        let above = self.above_branches.draw_inclusive(ctx.random);
        for cy in y..y + height {
            if allows(tree.may_replace, ctx.view.block(x, cy, z)) {
                ctx.view.set_block(x, cy, z, self.block);
            }
            // Asked at every log, placed or not.
            if let Some(feature) = self.log_decoration {
                FEATURES[feature].place(ctx, [x, cy, z]);
            }
        }
        let mut sides = SIDES;
        for i in 1..sides.len() {
            let j = (ctx.random.next_u32() % (i as u32 + 1)) as usize;
            if j != i {
                sides.swap(i, j);
            }
        }
        let branches = self.branches.draw_inclusive(ctx.random);
        let crown = y + height - above;
        for &(dx, dz) in sides.iter().take(branches.max(0) as usize) {
            if allows(tree.may_replace, ctx.view.block(x + dx, crown - 1, z + dz)) {
                ctx.view.set_block(x + dx, crown - 1, z + dz, if dx == 0 { self.block_z } else { self.block_x });
            }
        }
        tree.place_canopies(ctx, &[Anchor::at([x, crown, z])]);
        Some(origin)
    }
}

impl PoplarCanopy {
    pub fn place(&self, tree: &Tree, view: &mut View, random: &mut Mt19937, [x, y, z]: [i32; 3]) {
        let radius = self.radius(random) - 1;
        let height = self.height.draw_inclusive(random);
        let flip = random.next_boolean();
        let shape = Shape { canopy: self, tree, at: [x, y, z], height, flip };
        shape.layer(view, random, radius - 2, height - 1);
        shape.layer(view, random, radius - 1, height - 2);
        shape.layer(view, random, radius - 1, height - 3);
        for dy in (1..=height - 4).rev() {
            shape.layer(view, random, radius, dy);
        }
        // Branch logs along both axes under the widest layers, x first.
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                let on_axis = dx == 0 && radius - dz.abs() > 3 || dz == 0 && radius - dx.abs() > 3;
                if on_axis && allows(tree.may_replace, view.block(x + dx, y + height - 4, z + dz)) {
                    view.set_block(x + dx, y + height - 4, z + dz, if dz == 0 { self.branch_x } else { self.branch_z });
                }
            }
        }
        shape.layer(view, random, radius - 1, 0);
        let skirt = if radius < 3 { 1 } else if radius < 5 { radius - 2 } else { 2 };
        shape.layer(view, random, skirt, -1);
    }

    /// `nextInt` of the summed weights (none when 0), then the first value the remainder falls in.
    fn radius(&self, random: &mut Mt19937) -> i32 {
        let total: i32 = self.radius.iter().map(|&(_, weight)| weight).sum();
        let mut left = if total == 0 { 0 } else { random.next_int(total) };
        for &(value, weight) in self.radius {
            left -= weight;
            if left < 0 {
                return value;
            }
        }
        self.radius.last().map_or(1, |&(value, _)| value)
    }
}

#[cfg(test)]
mod tests {
    use crate::OverworldGenerator;
    use crate::block::Block;

    /// No BDS world to compare with: only that a camp's poplar element grows a tree of the documented shape.
    #[test]
    fn a_poplar_element_grows_a_trunk_branches_and_a_canopy() {
        let generator = OverworldGenerator::new(20261003);
        let grown = (-64..-56).flat_map(|cx| (4..12).map(move |cz| (cx, cz))).find_map(|(cx, cz)| {
            let shape = generator.carved(cx, cz);
            let y = (60..200).rev().find(|&y| shape.block(8, y, 8) != Block::Air)?;
            let at = [cx * 16 + 8, y + 1, cz * 16 + 8];
            let writes = generator.structure_feature_writes("orange_poplar_tree_feature", (cx, cz), at, 0);
            (shape.block(8, y, 8) == Block::GrassBlock && !writes.is_empty()).then_some((at, writes))
        });
        let ([x, y, z], writes) = grown.expect("a grass column where the tree fits");
        let count = |wanted: &dyn Fn([i32; 3], Block) -> bool| writes.iter().filter(|&&(at, block, _)| wanted(at, block)).count();
        let trunk = count(&|at, block| block == Block::PoplarLog && at[0] == x && at[2] == z);
        let lying = count(&|_, block| matches!(block, Block::PoplarLogX | Block::PoplarLogZ));
        let leaves = count(&|_, block| block == Block::OrangePoplarLeaves);
        let reach = writes.iter().map(|&(at, _, _)| (at[0] - x).abs().max((at[2] - z).abs())).max().unwrap();
        assert!((7..=10).contains(&trunk), "{trunk} trunk logs");
        assert!(lying >= 1 && leaves > 60 && reach <= 7, "{lying} lying logs, {leaves} leaves, reach {reach}");
        assert!(writes.iter().all(|&(at, _, _)| at[1] >= y - 1 && at[1] <= y + 12));
    }
}

struct Shape<'a> {
    canopy: &'a PoplarCanopy,
    tree: &'a Tree,
    at: [i32; 3],
    height: i32,
    flip: bool,
}

impl Shape<'_> {
    /// One leaf layer: a diamond of `radius - 1`, one wider in two opposite quadrants (which two,
    /// `flip` says) and where the hole roll passes; the two top layers are one narrower and skip
    /// their square's edge without rolling. One `nextFloat` per rolled cell, x outer.
    fn layer(&self, view: &mut View, random: &mut Mt19937, radius: i32, dy: i32) {
        let [x, y, z] = self.at;
        let top = dy == self.height - 2 || dy == self.height - 1;
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                if top && (dx.abs() == radius || dz.abs() == radius) {
                    continue;
                }
                let wide = if self.flip { dx < 0 && dz < 0 || dx > 0 && dz > 0 } else { dx > 0 && dz < 0 || dx < 0 && dz > 0 };
                let bonus = if wide { 1 } else { -i32::from(top) };
                let filled = i32::from(self.canopy.side_hole_chance < random.next_float());
                if dx.abs() + dz.abs() <= bonus + radius - 1 + filled && allows(self.tree.may_replace, view.block(x + dx, y + dy, z + dz)) {
                    view.set_block(x + dx, y + dy, z + dz, self.canopy.leaf);
                }
            }
        }
    }
}
