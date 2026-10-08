//! The world one source chunk's features see: its own and its neighbours' terrain before features,
//! changed by its own writes only, each write logged per target chunk for replay (see
//! docs/research/worldgen-bds.md, Feature placement).

use std::cell::RefCell;
use std::sync::Arc;

use super::ticks::TickQueue;
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
    /// The block source's tick queue (liquid/mod.rs).
    pub ticks: TickQueue,
    /// Run after each `set_block`: `setBlock`'s neighbour updates (liquid/mod.rs).
    pub on_write: Option<fn(&mut View, [i32; 3])>,
}

impl<'a> View<'a> {
    pub fn new(chunk: (i32, i32), load: &'a dyn Fn(i32, i32) -> Arc<ChunkShape>) -> Self {
        let slots = SIDE * SIDE;
        Self {
            chunk,
            load,
            shapes: RefCell::new(vec![None; slots]),
            writes: vec![Vec::new(); slots],
            tops: RefCell::new(vec![None; slots]),
            ticks: TickQueue::default(),
            on_write: None,
        }
    }

    pub fn chunk(&self) -> (i32, i32) {
        self.chunk
    }

    /// `BlockSource::hasBlock`: inside the world's height and the writable region.
    pub fn has_block(&self, x: i32, y: i32, z: i32) -> bool {
        self.locate(x, y, z).is_some()
    }

    /// The decorated chunk's own fluid ticks from generation, as world positions.
    pub fn own_fluid_ticks(&self) -> Vec<([i32; 3], bool)> {
        let slot = slot(0, 0).expect("centre");
        let (x0, z0) = (self.chunk.0 * 16, self.chunk.1 * 16);
        self.read(slot, 0);
        let shapes = self.shapes.borrow();
        let shape = shapes[slot].as_ref().expect("read just above");
        let at = |i: i32| [x0 + (i & 15), i / 256 + MIN_Y, z0 + ((i >> 4) & 15)];
        shape.fluid_ticks().iter().map(|&(index, lava)| (at(index as i32), lava)).collect()
    }

    /// `BlockSource::addToTickingQueue` (1415f50f0): a tick joins the block source's queue only when
    /// every chunk within 8 blocks of the cell is in the region; else it goes to its chunk's own queue,
    /// which runs in game, after decoration (`TickQueue::in_game`).
    pub fn schedule(&mut self, at: [i32; 3], lava: bool, delay: u64) {
        let near = |x: i32, z: i32| [x - 8, x + 8].iter().all(|&x| [z - 8, z + 8].iter().all(|&z| self.has_block(x, MIN_Y, z)));
        if self.ticks.game || near(at[0], at[2]) {
            self.ticks.add(at, lava, delay);
        } else {
            self.ticks.in_game.push((at, lava, delay));
        }
    }

    /// Whether every chunk within a block of the column is in the region (an instatick's range).
    pub fn can_tick(&self, x: i32, z: i32) -> bool {
        [x - 1, x + 1].iter().all(|&x| [z - 1, z + 1].iter().all(|&z| self.has_block(x, MIN_Y, z)))
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

    /// A feature's write (`setBlock` with flags 3): `set_block_quiet`, then `on_write`.
    pub fn set_block(&mut self, x: i32, y: i32, z: i32, block: Block) -> bool {
        let written = self.set_block_quiet(x, y, z, block);
        if let Some(hook) = self.on_write.filter(|_| written) {
            hook(self, [x, y, z]);
        }
        written
    }

    /// False, writing nothing, outside the world's height and the writable region, and when the
    /// block is already there (`BlockSource::setBlock` 1415f6ae0 reports whether the block changed).
    pub fn set_block_quiet(&mut self, x: i32, y: i32, z: i32, block: Block) -> bool {
        let Some((slot, index)) = self.locate(x, y, z) else { return false };
        let seen = self.read(slot, index);
        if seen == block {
            return false;
        }
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
    /// that blocks motion (not fluids, snow layers or leaves: oracle, grove pines stand under spruce
    /// canopies); the world's top outside the region, its bottom if none.
    pub fn above_top_solid(&self, x: i32, z: i32) -> i32 {
        let top = MIN_Y + HEIGHT as i32;
        if self.locate(x, MIN_Y, z).is_none() {
            return top;
        }
        let solid = |block: Block| block.blocks_motion() && !block.is_leaves();
        (MIN_Y..top).rev().find(|&y| solid(self.block(x, y, z))).map_or(MIN_Y, |y| y + 1)
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
