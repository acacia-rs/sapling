//! The world one source chunk's features see: its own and its neighbours' terrain before features,
//! changed by its own writes only, each write logged per target chunk for replay (see
//! docs/research/worldgen-bds.md, Feature placement).

use std::cell::RefCell;
use std::sync::Arc;

use crate::block::Block;
use crate::terrain::{ChunkShape, HEIGHT, MIN_Y};

/// How many chunks around the decorated one its features may write.
pub const REACH: i32 = 1;
const SIDE: usize = (2 * REACH + 1) as usize;

/// A write of `block` over `seen`, at `index` of the target column.
#[derive(Debug, Clone, Copy)]
pub struct Write {
    pub index: u32,
    pub block: Block,
    pub seen: Block,
}

/// One source chunk's writes, per target chunk around it.
pub struct ChunkWrites(Vec<Vec<Write>>);

impl ChunkWrites {
    /// The writes into the chunk `(dx, dz)` away from the source.
    pub fn towards(&self, dx: i32, dz: i32) -> &[Write] {
        slot(dx, dz).map_or(&[], |slot| &self.0[slot])
    }
}

fn slot(dx: i32, dz: i32) -> Option<usize> {
    (dx.abs() <= REACH && dz.abs() <= REACH).then(|| ((dx + REACH) as usize) * SIDE + (dz + REACH) as usize)
}

pub struct View<'a> {
    chunk: (i32, i32),
    load: &'a dyn Fn(i32, i32) -> Arc<ChunkShape>,
    shapes: RefCell<Vec<Option<Arc<ChunkShape>>>>,
    writes: Vec<Vec<Write>>,
}

impl<'a> View<'a> {
    pub fn new(chunk: (i32, i32), load: &'a dyn Fn(i32, i32) -> Arc<ChunkShape>) -> Self {
        Self { chunk, load, shapes: RefCell::new(vec![None; SIDE * SIDE]), writes: vec![Vec::new(); SIDE * SIDE] }
    }

    /// The slot and column index of a block, if it lies in the writable region.
    fn locate(&self, x: i32, y: i32, z: i32) -> Option<(usize, usize)> {
        if !(MIN_Y..MIN_Y + HEIGHT as i32).contains(&y) {
            return None;
        }
        let slot = slot(x.div_euclid(16) - self.chunk.0, z.div_euclid(16) - self.chunk.1)?;
        Some((slot, ChunkShape::index(x.rem_euclid(16) as usize, y, z.rem_euclid(16) as usize)))
    }

    fn read(&self, slot: usize, index: usize) -> Block {
        let mut shapes = self.shapes.borrow_mut();
        let shape = shapes[slot].get_or_insert_with(|| {
            let (dx, dz) = ((slot / SIDE) as i32 - REACH, (slot % SIDE) as i32 - REACH);
            (self.load)(self.chunk.0 + dx, self.chunk.1 + dz)
        });
        shape.block_at(index)
    }

    /// Air outside the world's height and the writable region.
    pub fn block(&self, x: i32, y: i32, z: i32) -> Block {
        self.locate(x, y, z).map_or(Block::Air, |(slot, index)| self.read(slot, index))
    }

    /// False, writing nothing, outside the world's height and the writable region.
    pub fn set_block(&mut self, x: i32, y: i32, z: i32, block: Block) -> bool {
        let Some((slot, index)) = self.locate(x, y, z) else { return false };
        let seen = self.read(slot, index);
        let mut shapes = self.shapes.borrow_mut();
        let shape = shapes[slot].as_mut().expect("read just above");
        Arc::make_mut(shape).set_at(index, block);
        self.writes[slot].push(Write { index: index as u32, block, seen });
        true
    }

    pub fn into_writes(self) -> ChunkWrites {
        ChunkWrites(self.writes)
    }
}
