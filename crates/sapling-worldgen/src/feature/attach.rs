//! `may_attach_to` of `minecraft:single_block_feature` (featurelab `features/single_block.go`,
//! MIT): hard checks above, below and on the diagonals, counted sides, and auto-rotation toward
//! the last side that matched.

use super::spec::Attach;
use super::view::View;
use crate::block::Block;

/// North, east, south, west.
const SIDES: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
const DIAGONALS: [(i32, i32); 4] = [(1, 1), (-1, 1), (1, -1), (-1, -1)];

/// True when every list is empty or `block` is in one of the non-empty ones.
fn matches(block: Block, lists: &[&[Block]]) -> bool {
    lists.iter().all(|list| list.is_empty()) || lists.iter().any(|list| list.contains(&block))
}

impl Attach {
    /// The block to place (rotated toward the last matching side when `auto_rotate`), or None.
    pub fn check(&self, view: &View, [x, y, z]: [i32; 3], block: Block) -> Option<Block> {
        if !matches(view.block(x, y + 1, z), &[self.top, self.all]) || !matches(view.block(x, y - 1, z), &[self.bottom, self.all]) {
            return None;
        }
        if DIAGONALS.iter().any(|&(dx, dz)| !matches(view.block(x + dx, y, z + dz), &[self.diagonal, self.all])) {
            return None;
        }
        let mut matched = 0;
        let mut result = block;
        for (side, &(dx, dz)) in SIDES.iter().enumerate() {
            if matches(view.block(x + dx, y, z + dz), &[self.side[side], self.sides, self.all]) {
                matched += 1;
                result = if self.auto_rotate { rotated(block, side) } else { block };
            }
        }
        (matched >= self.min_sides).then_some(result)
    }
}

/// `block` turned toward the side (north, east, south, west) it hangs on; cocoa's `direction` is
/// the side of its log (0 south, 1 west, 2 north, 3 east).
fn rotated(block: Block, side: usize) -> Block {
    const COCOA: [[Block; 4]; 3] = [
        [Block::Cocoa0South, Block::Cocoa0West, Block::Cocoa0North, Block::Cocoa0East],
        [Block::Cocoa1South, Block::Cocoa1West, Block::Cocoa1North, Block::Cocoa1East],
        [Block::Cocoa2South, Block::Cocoa2West, Block::Cocoa2North, Block::Cocoa2East],
    ];
    let Some(age) = COCOA.iter().position(|ages| ages.contains(&block)) else { return block };
    // North, east, south, west as cocoa directions.
    COCOA[age][[2, 3, 0, 1][side]]
}
