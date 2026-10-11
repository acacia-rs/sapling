//! What `LiquidBlock::_isLiquidBlocking` (148b5ee50) reads off a block for water: its liquid
//! detection's `on_liquid_touches`, blocking or not (docs/research/worldgen-bds-liquids.md, "Blocking").

use std::collections::HashSet;
use std::sync::LazyLock;

use super::detect_table::GIVES_WAY;
use crate::block::Block;

static OPEN: LazyLock<HashSet<&str>> = LazyLock::new(|| GIVES_WAY.split_whitespace().collect());
static BY_INDEX: LazyLock<Vec<bool>> = LazyLock::new(|| Block::ALL.iter().map(|b| !b.is_fluid() && !OPEN.contains(b.name())).collect());

/// Whether water stops at the block instead of flowing into its cell.
pub fn holds_water(block: Block) -> bool {
    match block {
        Block::Template(_) => !OPEN.contains(block.name()),
        _ => BY_INDEX[block.index()],
    }
}
