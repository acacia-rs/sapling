//! What one source chunk's view (view.rs) leaves behind: its writes per target chunk, for replay.

use super::reach::{Cells, Tries};
use crate::block::Block;
use crate::block_entity::BlockEntity;
use crate::terrain::MIN_Y;

/// How many chunks around the decorated one its features may write.
pub const REACH: i32 = 1;
pub(super) const SIDE: usize = (2 * REACH + 1) as usize;

/// A write of `block` over `seen`, at `index` of the target column.
#[derive(Debug, Clone, Copy)]
pub struct Write {
    pub index: u32,
    pub block: Block,
    pub seen: Block,
}

/// A liquid tick left on its cell's chunk queue: cell, lava, delay.
pub type Routed = ([i32; 3], bool, u64);

/// One source chunk's writes, per target chunk around it, the ticks it left on chunk queues, what
/// `reach` followed, the instaticks it left on its own queue (`View::left`), and the block entities it
/// filed (world cell).
pub struct ChunkWrites(
    pub(super) Vec<Vec<Write>>,
    pub(super) Vec<Routed>,
    pub(super) Option<(Vec<Option<Cells>>, Tries)>,
    pub(super) Vec<Routed>,
    pub(super) Vec<([i32; 3], BlockEntity)>,
);

impl ChunkWrites {
    /// The block entities this source filed in `chunk`, in order.
    pub fn entities_in(&self, chunk: (i32, i32)) -> impl Iterator<Item = ([i32; 3], BlockEntity)> + '_ {
        self.4.iter().copied().filter(move |(at, _)| (at[0].div_euclid(16), at[2].div_euclid(16)) == chunk)
    }

    /// The ticks this source left on `chunk`'s queue, in the order they came.
    pub fn routed_to(&self, chunk: (i32, i32)) -> impl Iterator<Item = Routed> + '_ {
        self.1.iter().copied().filter(move |(at, _, _)| (at[0].div_euclid(16), at[2].div_euclid(16)) == chunk)
    }

    /// The cells of the chunk `(dx, dz)` away that this source left in doubt (reach.rs).
    pub fn doubt_towards(&self, dx: i32, dz: i32) -> Option<&Cells> {
        self.2.as_ref().zip(slot(dx, dz)).and_then(|((cells, _), slot)| cells[slot].as_ref())
    }

    pub fn tries(&self) -> Tries {
        self.2.as_ref().map_or_else(Tries::default, |&(_, tries)| tries)
    }

    /// The writes into the chunk `(dx, dz)` away from the source.
    pub fn towards(&self, dx: i32, dz: i32) -> &[Write] {
        slot(dx, dz).map_or(&[], |slot| &self.0[slot])
    }

    /// Every write in order of its target chunk, as world (x, y, z), block, seen.
    pub fn world_writes(&self, source: (i32, i32)) -> Vec<([i32; 3], Block, Block)> {
        let mut out = Vec::new();
        for (slot, writes) in self.0.iter().enumerate() {
            let (dx, dz) = ((slot / SIDE) as i32 - REACH, (slot % SIDE) as i32 - REACH);
            let (x0, z0) = ((source.0 + dx) * 16, (source.1 + dz) * 16);
            for w in writes {
                let i = w.index as i32;
                out.push(([x0 + (i & 15), i / 256 + MIN_Y, z0 + ((i >> 4) & 15)], w.block, w.seen));
            }
        }
        out
    }
}

pub(super) fn slot(dx: i32, dz: i32) -> Option<usize> {
    (dx.abs() <= REACH && dz.abs() <= REACH).then(|| ((dx + REACH) as usize) * SIDE + (dz + REACH) as usize)
}
