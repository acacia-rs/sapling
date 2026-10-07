//! `acacia_trunk` (acacias, dark and pale oaks): a `width`-square column leaning `lean_steps`
//! cells one way from `height - lean_height` up, then branches (leaning or vertical by
//! `allow_diagonal_growth`). featurelab `placeShapedTrunk`, `placeAcacia*Branches`.

use super::Anchor;
use super::common::valid_tree_position;
use super::spec::{AcaciaBranches, AcaciaTrunk, Tree};
use crate::feature::place::Ctx;

/// Per direction (south, west, north, east) the x and z steps.
const LEAN: [(i32, i32); 4] = [(0, 1), (-1, 0), (0, -1), (1, 0)];

/// The trunk cell's sides (west, east, north, south) a decoration may use; a one-wide trunk
/// enables west and north only, as BDS tests the minus edge first.
fn mask(dx: i32, dz: i32, width: i32) -> [bool; 4] {
    [dx == 0, dx != 0 && dx == width - 1, dz == 0, dz != 0 && dz == width - 1]
}

impl AcaciaTrunk {
    pub fn place(&self, tree: &Tree, ctx: &mut Ctx, origin: [i32; 3]) -> Option<[i32; 3]> {
        let height = self.height_base + self.height_intervals.iter().map(|&i| ctx.random.next_int(i)).sum::<i32>();
        let origin = tree.roots_then(ctx, origin, height)?;
        if height < 1 || !tree.prepare(ctx.view, origin, height, 1) {
            return None;
        }
        let [ox, oy, oz] = origin;
        for dx in 0..self.width {
            for dz in 0..self.width {
                tree.place_base_block(ctx.view, [ox + dx, oy - 1, oz + dz]);
            }
        }
        let direction = ctx.random.next_int(4) as usize;
        let lean_start = height - self.lean_height.draw(ctx.random);
        let mut lean_steps = self.lean_steps.draw(ctx.random);
        let lean_length = self.lean_length.draw(ctx.random);
        let (mut x, mut z) = (ox, oz);
        let mut candidates = Vec::new();
        for i in 0..height + lean_length {
            if i >= lean_start && lean_steps > 0 {
                lean_steps -= 1;
                x += LEAN[direction].0;
                z += LEAN[direction].1;
            }
            let y = oy + i.min(height - 1);
            // 143bd34c0: the leaning column gates the layer; without diagonal growth logs stay over the origin.
            if !valid_tree_position(ctx.view, [x, y, z], tree.may_replace) {
                continue;
            }
            let (cx, cy, cz) = if self.allow_diagonal { (x, y, z) } else { (ox, oy + i, oz) };
            for dx in 0..self.width {
                for dz in 0..self.width {
                    let p = [cx + dx, cy, cz + dz];
                    if !valid_tree_position(ctx.view, p, tree.may_replace) {
                        continue;
                    }
                    ctx.view.set_block(p[0], p[1], p[2], self.block);
                    if let Some(decoration) = &self.decoration {
                        decoration.place(ctx.view, p, mask(dx, dz, self.width), ctx.random);
                    }
                    if i >= self.min_height_for_canopy {
                        candidates.push(p);
                    }
                }
            }
        }
        // A trunk that never cleared min_height_for_canopy hands its canopy a zeroed anchor (draws still happen).
        let anchor = candidates.last().copied().unwrap_or([0; 3]);
        tree.place_canopies(ctx, &[Anchor { at: anchor, candidates }]);
        if self.allow_diagonal {
            self.branches.leaning(self, tree, ctx, origin, height, direction, lean_start);
        } else {
            self.branches.vertical(self, tree, ctx, origin, anchor);
        }
        Some(origin)
    }
}

impl AcaciaBranches {
    /// One branch at most, away from the lean: drawn direction (always), chance, position then
    /// length; the walk steps sideways every cell even where the log is refused.
    #[allow(clippy::too_many_arguments)]
    fn leaning(&self, trunk: &AcaciaTrunk, tree: &Tree, ctx: &mut Ctx, origin: [i32; 3], height: i32, lean: usize, lean_start: i32) {
        let direction = ctx.random.next_int(4) as usize;
        if direction == lean || !self.chance.roll(ctx.random) {
            return;
        }
        let position = self.position.draw(ctx.random);
        let mut length = self.length.draw(ctx.random);
        let mut index = lean_start - position;
        if index >= height || length < 1 {
            return;
        }
        let (mut x, mut z) = (origin[0], origin[2]);
        let mut last_y = None;
        loop {
            if index >= 1 {
                x += LEAN[direction].0;
                z += LEAN[direction].1;
                let y = origin[1] + index;
                if valid_tree_position(ctx.view, [x, y, z], tree.may_replace) {
                    ctx.view.set_block(x, y, z, trunk.block);
                    if let Some(decoration) = &trunk.decoration {
                        decoration.place(ctx.view, [x, y, z], [true; 4], ctx.random);
                    }
                    last_y = Some(y);
                }
            }
            index += 1;
            if index >= height || length <= 1 {
                break;
            }
            length -= 1;
        }
        if let (Some(y), Some(canopy)) = (last_y, self.canopy) {
            canopy.place(tree, ctx.view, ctx.random, &Anchor::at([x, y, z]));
        }
    }

    /// Per cell of the ring around the trunk: chance, length then position, a column down from
    /// `anchor - position` (no decoration), and the branch canopy even for an empty column.
    fn vertical(&self, trunk: &AcaciaTrunk, tree: &Tree, ctx: &mut Ctx, origin: [i32; 3], anchor: [i32; 3]) {
        let width = trunk.width;
        for dx in -1..=width {
            for dz in -1..=width {
                let ring = dx < 0 || dx >= width || dz < 0 || dz >= width;
                if !ring || !self.chance.roll(ctx.random) {
                    continue;
                }
                let length = self.length.draw(ctx.random);
                let position = self.position.draw(ctx.random);
                let (x, z) = (origin[0] + dx, origin[2] + dz);
                for n in 0..length.max(0) {
                    let y = anchor[1] - position - n;
                    if valid_tree_position(ctx.view, [x, y, z], tree.may_replace) {
                        ctx.view.set_block(x, y, z, trunk.block);
                    }
                }
                if let Some(canopy) = self.canopy {
                    canopy.place(tree, ctx.view, ctx.random, &Anchor::at([anchor[0] + dx, anchor[1] - position, anchor[2] + dz]));
                }
            }
        }
    }
}
