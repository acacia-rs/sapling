//! The lava lake of `decorationPostProcessChunk` (1482143a0) as the first thing a source does: it
//! reads the 3x3 as earlier sources left it and what it carves is there for the monster rooms,
//! the feature rules and the liquid pass (docs/research/worldgen-bds-order.md, "replay5").

use super::view::View;
use crate::block::Block;
use crate::decoration::lake;
use crate::random::Mt19937;
use crate::terrain::CellKind;

fn kind(block: Block) -> CellKind {
    match block {
        Block::Air => CellKind::Air,
        block if block.is_lava() => CellKind::Lava,
        block if block.is_fluid() => CellKind::Water,
        _ => CellKind::Solid,
    }
}

/// `random` is the chunk's post-process generator; the monster rooms take it on.
pub fn lava_lake(random: &mut Mt19937, chunk: (i32, i32), view: &mut View) {
    let Some(lake) = lake::lava_lake(random, chunk.0, chunk.1, &mut |x, y, z| kind(view.block(x, y, z))) else { return };
    super::trace::line(|| format!("{chunk:?} lava lake at {:?}", lake.corner()));
    for cx in chunk.0 - 1..=chunk.0 + 1 {
        for cz in chunk.1 - 1..=chunk.1 + 1 {
            lake.apply(cx, cz, |x, y, z, block| _ = view.set_block(cx * 16 + x as i32, y, cz * 16 + z as i32, block));
        }
    }
}
