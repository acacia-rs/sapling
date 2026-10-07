//! The simple trunk (bare `trunk` key): a straight column, optionally sunk through
//! `may_grow_through` (`can_be_submerged`), canopy one above the top log.

use super::common::allows;
use super::spec::{SimpleTrunk, Tree};
use super::Anchor;
use crate::block::Block;
use crate::feature::place::Ctx;

impl SimpleTrunk {
    pub fn place(&self, tree: &Tree, ctx: &mut Ctx, origin: [i32; 3]) -> Option<[i32; 3]> {
        let height = self.height.draw(ctx.random);
        let origin = tree.roots_then(ctx, origin, height)?;
        let [x, y0, z] = origin;
        let mut y = y0;
        // Without may_grow_through the descent still stops at the ground (oracle: swamp oaks stand on the bottom).
        let sinks = |block: Block| if tree.may_grow_through.is_empty() { block == Block::Water } else { tree.may_grow_through.contains(&block) };
        if self.max_depth >= 1 && sinks(ctx.view.block(x, y - 1, z)) {
            let mut budget = self.max_depth;
            loop {
                budget -= 1;
                y -= 1;
                if budget <= 0 || !sinks(ctx.view.block(x, y - 1, z)) {
                    break;
                }
            }
        }
        let base = [x, y, z];
        if !tree.prepare(ctx.view, base, height, 1) {
            return None;
        }
        let total = height + self.height_modifier.draw(ctx.random);
        if total < 1 {
            return None;
        }
        let mut top = None;
        for cy in y..y + total {
            let gate = if cy >= y0 { tree.may_replace } else { tree.may_grow_through };
            if allows(gate, ctx.view.block(x, cy, z)) {
                ctx.view.set_block(x, cy, z, self.block);
                if let Some(decoration) = &self.decoration {
                    decoration.place(ctx.view, [x, cy, z], [true; 4], ctx.random);
                }
                top = Some(cy);
            }
        }
        let top = top?;
        tree.place_canopies(ctx, &[Anchor { at: [x, top + 1, z], candidates: Vec::new() }]);
        tree.place_base_block(ctx.view, [x, y - 1, z]);
        // BDS returns one above the top log: a sequence's next feature (jungle cocoa) starts there.
        Some([x, top + 1, z])
    }
}
