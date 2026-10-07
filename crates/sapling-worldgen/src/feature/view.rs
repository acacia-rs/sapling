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
    /// Per slot, `highest_block` once asked.
    tops: RefCell<Vec<Option<i32>>>,
}

impl<'a> View<'a> {
    pub fn new(chunk: (i32, i32), load: &'a dyn Fn(i32, i32) -> Arc<ChunkShape>) -> Self {
        Self { chunk, load, shapes: RefCell::new(vec![None; SIDE * SIDE]), writes: vec![Vec::new(); SIDE * SIDE], tops: RefCell::new(vec![None; SIDE * SIDE]) }
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
        if let Some(top) = &mut self.tops.get_mut()[slot]
            && block != Block::Air
        {
            *top = (*top).max(y);
        }
        true
    }

    /// `query.above_top_solid` (`LevelChunk::getAboveTopSolidBlock` 140a08bb0): one above the highest block
    /// that blocks motion (not fluids or snow layers); the world's top outside the region, its bottom if none.
    pub fn above_top_solid(&self, x: i32, z: i32) -> i32 {
        let top = MIN_Y + HEIGHT as i32;
        if self.locate(x, MIN_Y, z).is_none() {
            return top;
        }
        (MIN_Y..top).rev().find(|&y| self.block(x, y, z).blocks_motion()).map_or(MIN_Y, |y| y + 1)
    }

    /// `query.heightmap` (lambda 1411a2740): the chunk's light heightmap, one above the highest block
    /// that is neither air nor a see-through plant (water counts); the world's top outside the region.
    pub fn heightmap(&self, x: i32, z: i32) -> i32 {
        let top = MIN_Y + HEIGHT as i32;
        if self.locate(x, MIN_Y, z).is_none() {
            return top;
        }
        let blocks_light = |block: Block| block != Block::Air && (!block.is_plant() || block.is_crop_block()) && !block.is_leaves();
        (MIN_Y..top).rev().find(|&y| blocks_light(self.block(x, y, z))).map_or(MIN_Y, |y| y + 1)
    }

    /// `LevelChunk` +0xf88 plus the minimum y: the highest non-air block of the chunk holding
    /// `(x, z)`, raised by this view's writes; None outside the region.
    pub fn highest_block(&self, x: i32, z: i32) -> Option<i32> {
        let (slot, _) = self.locate(x, MIN_Y, z)?;
        if let Some(top) = self.tops.borrow()[slot] {
            return Some(top);
        }
        let (cx, cz) = (x.div_euclid(16) * 16, z.div_euclid(16) * 16);
        let top = (MIN_Y..MIN_Y + HEIGHT as i32)
            .rev()
            .find(|&y| (0..16).any(|dx| (0..16).any(|dz| self.block(cx + dx, y, cz + dz) != Block::Air)))
            .unwrap_or(MIN_Y);
        self.tops.borrow_mut()[slot] = Some(top);
        Some(top)
    }

    pub fn into_writes(self) -> ChunkWrites {
        ChunkWrites(self.writes)
    }
}
