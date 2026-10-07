//! The world as one chunk's structure post-process sees it (BDS's `BlockSource` over the 3x3
//! chunks): writes land in the chunk only, reads may reach the neighbours' loaded terrain.

use std::cell::RefCell;
use std::collections::HashMap;

use super::BoundingBox;
use crate::block::Block;
use crate::terrain::{ChunkShape, MIN_Y};

const TOP: i32 = 319;

pub struct Canvas<'a> {
    shape: &'a mut ChunkShape,
    chunk: (i32, i32),
    pub bb: BoundingBox,
    neighbour: &'a dyn Fn(i32, i32, i32) -> Block,
    /// Light heightmap per column, as `LevelChunk` +0xb80 keeps it.
    heights: RefCell<HashMap<(i32, i32), i32>>,
    /// Writes outside the chunk kept (and read back) while a piece placed whole is being drawn.
    spill: Option<(BoundingBox, HashMap<(i32, i32, i32), Block>)>,
}

impl<'a> Canvas<'a> {
    pub fn new(shape: &'a mut ChunkShape, chunk: (i32, i32), neighbour: &'a dyn Fn(i32, i32, i32) -> Block) -> Self {
        Self { shape, chunk, bb: BoundingBox::chunk(chunk.0, chunk.1), neighbour, heights: RefCell::new(HashMap::new()), spill: None }
    }

    /// Until `end_spill`, writes inside `area` but outside the chunk are kept for later reads, as
    /// the 3x3 `BlockSource` of a post-process placing a whole piece shows them.
    pub fn begin_spill(&mut self, area: BoundingBox) {
        self.spill = Some((area, HashMap::new()));
    }

    pub fn end_spill(&mut self) {
        self.spill = None;
    }

    fn inside(&self, x: i32, z: i32) -> bool {
        x >> 4 == self.chunk.0 && z >> 4 == self.chunk.1
    }

    /// Any block: the chunk's own, a neighbour's, air outside the world's heights.
    pub fn get(&self, x: i32, y: i32, z: i32) -> Block {
        if !(MIN_Y..=TOP).contains(&y) {
            Block::Air
        } else if self.inside(x, z) {
            self.shape.block((x & 15) as usize, y, (z & 15) as usize)
        } else if let Some(&block) = self.spill.as_ref().and_then(|(_, cells)| cells.get(&(x, y, z))) {
            block
        } else {
            (self.neighbour)(x, y, z)
        }
    }

    /// A block inside the chunk; air elsewhere (BDS's pieces read through the chunk box).
    pub fn get_clipped(&self, x: i32, y: i32, z: i32) -> Block {
        if self.bb.contains(x, y, z) { self.get(x, y, z) } else { Block::Air }
    }

    /// Writes inside the chunk only.
    pub fn set(&mut self, x: i32, y: i32, z: i32, block: Block) {
        if self.bb.contains(x, y, z) {
            self.shape.set_block((x & 15) as usize, y, (z & 15) as usize, block);
            let mut heights = self.heights.borrow_mut();
            if let Some(h) = heights.get_mut(&(x, z))
                && counts_for_height(block)
            {
                *h = (*h).max(y + 1);
            }
        } else if let Some((area, cells)) = &mut self.spill
            && area.contains(x, y, z)
        {
            cells.insert((x, y, z), block);
        }
    }

    /// One above the highest block that is neither air nor a see-through plant.
    pub fn height(&self, x: i32, z: i32) -> i32 {
        if let Some(&h) = self.heights.borrow().get(&(x, z)) {
            return h;
        }
        let h = (MIN_Y..=TOP).rev().find(|&y| counts_for_height(self.get(x, y, z))).map_or(MIN_Y, |y| y + 1);
        self.heights.borrow_mut().insert((x, z), h);
        h
    }

    /// One above the highest solid block, fluids passed (`BlockSource` vt+0x178).
    pub fn floor_height(&self, x: i32, z: i32) -> i32 {
        (MIN_Y..=TOP).rev().find(|&y| self.get(x, y, z).is_solid()).map_or(MIN_Y, |y| y + 1)
    }

    /// Below the light heightmap (BDS's structure "interior" test); never at the world's top.
    pub fn interior(&self, x: i32, y: i32, z: i32) -> bool {
        y < TOP + 1 && (y < MIN_Y || self.height(x, z) > y)
    }
}

fn counts_for_height(block: Block) -> bool {
    block != Block::Air && !block.is_plant()
}
