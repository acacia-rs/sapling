//! One block column as a surface builder sees it: the blocks from `start` down to (not
//! including) `lower`.

use crate::block::Block;
use crate::terrain::ChunkShape;

pub struct Column<'a> {
    pub shape: &'a mut ChunkShape,
    pub x: usize,
    pub z: usize,
    /// World block coordinates of the column.
    pub world: (i32, i32),
    pub start: i32,
    pub lower: i32,
}

impl Column<'_> {
    pub fn get(&self, y: i32) -> Block {
        self.shape.block(self.x, y, self.z)
    }

    pub fn set(&mut self, y: i32, block: Block) {
        self.shape.set_block(self.x, y, self.z, block);
    }
}
