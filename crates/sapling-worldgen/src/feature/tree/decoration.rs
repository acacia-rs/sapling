//! `minecraft:horizontal_tree_decoration_feature` (place 149e8ca80; shelf mushrooms on poplar
//! logs): the block on one random side of the log it is placed at. Unverified, as poplar.rs.

use super::fallen::FACINGS;
use super::spec::HorizontalDecoration;
use crate::block::Block;
use crate::feature::place::Ctx;

/// The six cells around the decoration and the four around its log that `allow_adjacent` reads.
const AROUND: [(i32, i32, i32); 6] = [(0, 1, 0), (0, -1, 0), (0, 0, -1), (0, 0, 1), (-1, 0, 0), (1, 0, 0)];

impl HorizontalDecoration {
    /// `nextInt(4)` picks the side; in air, off the log's ends and (unless adjacent ones are
    /// allowed) with no other of its kind beside it or its log, `nextInt(2)` picks the growth.
    pub fn place(&self, ctx: &mut Ctx, [x, y, z]: [i32; 3]) -> Option<[i32; 3]> {
        let side = ctx.random.next_int(4) as usize;
        let (dx, dz) = FACINGS[side];
        let (tx, tz) = (x + dx, z + dz);
        let own = |block: Block| self.blocks.iter().flatten().any(|&b| b == block);
        let open = ctx.view.block(tx, y, tz) == Block::Air;
        let beside_log = FACINGS.iter().any(|&(ox, oz)| own(ctx.view.block(x + ox, y, z + oz)));
        let beside = AROUND.iter().any(|&(ox, oy, oz)| own(ctx.view.block(tx + ox, y + oy, tz + oz)));
        let log = ctx.view.block(x, y, z).states();
        let bark = !self.bark_side_only || if log.contains("pillar_axis=x") { dx == 0 } else if log.contains("pillar_axis=z") { dz == 0 } else { log.contains("pillar_axis=y") };
        if !open || !bark || !self.allow_adjacent && (beside_log || beside) {
            return None;
        }
        let growth = ctx.random.next_int(2) as usize;
        ctx.view.set_block(tx, y, tz, self.blocks[side][growth]).then_some([tx, y, tz])
    }
}
