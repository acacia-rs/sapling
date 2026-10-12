//! The world one source chunk's features see: its own and its neighbours' terrain before features,
//! changed by its own writes only, each write logged per target chunk for replay (see
//! docs/research/worldgen-bds.md, Feature placement).

use std::cell::RefCell;
use std::sync::Arc;

use super::reach::{Cells, Reach, Tries};
use super::ticks::TickQueue;
use crate::block::Block;
use crate::random::Mt19937;
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

/// A liquid tick left on its cell's chunk queue: cell, lava, delay.
pub type Routed = ([i32; 3], bool, u64);

/// One source chunk's writes, per target chunk around it, the ticks it left on chunk queues, and
/// what `reach` followed.
pub struct ChunkWrites(Vec<Vec<Write>>, Vec<Routed>, Option<(Vec<Option<Cells>>, Tries)>);

impl ChunkWrites {
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

fn slot(dx: i32, dz: i32) -> Option<usize> {
    (dx.abs() <= REACH && dz.abs() <= REACH).then(|| ((dx + REACH) as usize) * SIDE + (dz + REACH) as usize)
}

/// What `recomputeHeightMap` (1409e29f0) counts: a block or liquid layer that absorbs light
/// (`Block` +0xa5). Leaves and water do; plants, bamboo, cacti, mangrove roots, snow layers and
/// speleothems do not (the saved heightmaps of the oracle worlds, examples/saved_heightmaps.rs).
fn blocks_light(block: Block) -> bool {
    let clear = matches!(block, Block::Bamboo | Block::BambooSmallLeaves | Block::BambooLargeLeaves | Block::Cactus | Block::MangroveRoots) || block.is_snow_layer();
    let thin = block.is_plant() && !block.is_crop_block() || block.is_speleothem() || clear;
    block != Block::Air && (!thin || block.is_waterlogged())
}

/// A chunk's pre-worldgen heightmap, None once its decoration has ended.
type Tops<'a> = &'a dyn Fn(i32, i32) -> Option<Arc<[i16; 256]>>;

pub struct View<'a> {
    chunk: (i32, i32),
    load: &'a dyn Fn(i32, i32) -> Arc<ChunkShape>,
    shapes: RefCell<Vec<Option<Arc<ChunkShape>>>>,
    writes: Vec<Vec<Write>>,
    /// Per slot, `highest_block` once asked.
    tops: RefCell<Vec<Option<i32>>>,
    /// The chunks as their last `recomputeHeightMap` saw them.
    frozen: Option<&'a dyn Fn(i32, i32) -> Arc<ChunkShape>>,
    /// Per slot, the heightmap columns of `frozen` asked so far (`i16::MIN`: not yet).
    heights: RefCell<Vec<Option<Box<[i16; 256]>>>>,
    generated: Option<Tops<'a>>,
    /// The block source's tick queue (liquid/mod.rs).
    pub ticks: TickQueue,
    /// The ticks decoration left on chunk queues (liquid/mod.rs).
    pub routed: Vec<Routed>,
    /// The oracle's taint analysis, when the generator tracks it.
    pub reach: Option<Reach<'a>>,
    /// BDS's per-thread generator, when the run it models is known (thread.rs).
    pub thread_random: Option<Mt19937>,
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
            frozen: None,
            heights: RefCell::new(vec![None; slots]),
            generated: None,
            ticks: TickQueue::default(),
            routed: Vec::new(),
            reach: None,
            thread_random: None,
        }
    }

    /// Heightmaps come from `frozen`'s chunks, whatever this view writes.
    pub fn with_frozen_heights(mut self, frozen: &'a dyn Fn(i32, i32) -> Arc<ChunkShape>) -> Self {
        self.frozen = Some(frozen);
        self
    }

    /// `generated_top` reads the chunks `tops` knows (those still to finish decorating).
    pub fn with_generated_tops(mut self, tops: Tops<'a>) -> Self {
        self.generated = Some(tops);
        self
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
        let queued = self.ticks.game || near(at[0], at[2]);
        super::trace::line(|| format!("    {} {} {} {} in {delay}", if queued { "queue" } else { "route" }, at[0], at[1], at[2]));
        if queued {
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
    pub(super) fn locate(&self, x: i32, y: i32, z: i32) -> Option<(usize, usize)> {
        if !(MIN_Y..MIN_Y + HEIGHT as i32).contains(&y) {
            return None;
        }
        let slot = slot(x.div_euclid(16) - self.chunk.0, z.div_euclid(16) - self.chunk.1)?;
        Some((slot, ChunkShape::index(x.rem_euclid(16) as usize, y, z.rem_euclid(16) as usize)))
    }

    fn read(&self, slot: usize, index: usize) -> Block {
        let (dx, dz) = ((slot / SIDE) as i32 - REACH, (slot % SIDE) as i32 - REACH);
        let chunk = (self.chunk.0 + dx, self.chunk.1 + dz);
        if let Some(reach) = &self.reach {
            reach.read(slot, index, chunk);
        }
        let mut shapes = self.shapes.borrow_mut();
        let shape = shapes[slot].get_or_insert_with(|| (self.load)(chunk.0, chunk.1));
        shape.block_at(index)
    }

    /// Air outside the world's height and the writable region.
    pub fn block(&self, x: i32, y: i32, z: i32) -> Block {
        self.locate(x, y, z).map_or(Block::Air, |(slot, index)| self.read(slot, index))
    }

    /// False, writing nothing, outside the world's height and the writable region, and when the
    /// block is already there (`BlockSource::setBlock` 1415f6ae0 reports whether the block changed).
    /// No neighbour hears of it: docs/research/worldgen-bds-liquids.md, "Feature writes wake nothing".
    pub fn set_block(&mut self, x: i32, y: i32, z: i32, block: Block) -> bool {
        let Some((slot, index)) = self.locate(x, y, z) else { return false };
        let seen = self.read(slot, index);
        // The liquid layer of a waterlogged plant stays under a waterloggable block.
        let block = if seen == Block::Water || seen.is_waterlogged() { block.into_water() } else { block };
        // A vein or lichen taken out of a water source leaves the water (one-worker run: worldgen-bds-order.md).
        let block = if block == Block::Air && seen.multiface().is_some() && seen.is_waterlogged() { Block::Water } else { block };
        if seen == block {
            return false;
        }
        let mut shapes = self.shapes.borrow_mut();
        let shape = shapes[slot].as_mut().expect("read just above");
        Arc::make_mut(shape).set_at(index, block);
        self.writes[slot].push(Write { index: index as u32, block, seen });
        if let Some(reach) = &mut self.reach {
            reach.wrote(slot, index);
        }
        super::trace::line(|| format!("    write {x} {y} {z} {block:?} over {seen:?}"));
        if let (Some(top), true) = (&mut self.tops.get_mut()[slot], block != Block::Air) {
            *top = (*top).max(y);
        }
        true
    }

    /// `query.above_top_solid` (`LevelChunk::getAboveTopSolidBlock` 140a08bb0): one above the highest block
    /// whose material blocks motion (its byte +3: a coral fan counts, a snow layer does not), leaves left out
    /// (oracle: grove pines stand under spruce canopies); the world's top outside the region, its bottom if none.
    pub fn above_top_solid(&self, x: i32, z: i32) -> i32 {
        let top = MIN_Y + HEIGHT as i32;
        if self.locate(x, MIN_Y, z).is_none() {
            return top;
        }
        let solid = |block: Block| block.material_blocks_motion() && !block.is_leaves();
        (MIN_Y..top).rev().find(|&y| solid(self.block(x, y, z))).map_or(MIN_Y, |y| y + 1)
    }

    /// `query.heightmap` (lambda 1411a2740): the chunk's light heightmap, one above the highest block
    /// that `blocks_light`; the world's top outside the region.
    /// Read off the `frozen` chunks when given: see worldgen-bds-features.md, "Heightmaps".
    pub fn heightmap(&self, x: i32, z: i32) -> i32 {
        let top = MIN_Y + HEIGHT as i32;
        let Some((slot, _)) = self.locate(x, MIN_Y, z) else { return top };
        let Some(frozen) = self.frozen else {
            return (MIN_Y..top).rev().find(|&y| blocks_light(self.block(x, y, z))).map_or(MIN_Y, |y| y + 1);
        };
        let mut maps = self.heights.borrow_mut();
        let (lx, lz) = (x.rem_euclid(16) as usize, z.rem_euclid(16) as usize);
        let column = &mut maps[slot].get_or_insert_with(|| Box::new([i16::MIN; 256]))[lz * 16 + lx];
        if *column == i16::MIN {
            let (dx, dz) = ((slot / SIDE) as i32 - REACH, (slot % SIDE) as i32 - REACH);
            let shape = frozen(self.chunk.0 + dx, self.chunk.1 + dz);
            *column = (MIN_Y..top).rev().find(|&y| blocks_light(shape.block(lx, y, lz))).map_or(MIN_Y, |y| y + 1) as i16;
        }
        *column as i32
    }

    /// The pre-worldgen heightmap (`LevelChunk` +0xf80) of a chunk whose decoration has not ended, else
    /// `heightmap`: what `LargeSpeleothem::placeBlocks` caps a stalagmite's columns with.
    pub fn generated_top(&self, x: i32, z: i32) -> i32 {
        let tops = self.locate(x, MIN_Y, z).and(self.generated).and_then(|tops| tops(x.div_euclid(16), z.div_euclid(16)));
        tops.map_or_else(|| self.heightmap(x, z), |tops| tops[(z.rem_euclid(16) * 16 + x.rem_euclid(16)) as usize] as i32)
    }

    /// `LevelChunk` +0xf88 plus the minimum y, for the chunk holding `(x, z)`: the highest value of its
    /// heightmap when that was built (one above a block), raised to the y of every block set since
    /// (`LevelChunk::setBlock` 1409f59e0), this view's writes included; None outside the region.
    pub fn highest_block(&self, x: i32, z: i32) -> Option<i32> {
        let (slot, _) = self.locate(x, MIN_Y, z)?;
        if let Some(top) = self.tops.borrow()[slot] {
            return Some(top);
        }
        let (cx, cz) = (x.div_euclid(16) * 16, z.div_euclid(16) * 16);
        let set = (MIN_Y..MIN_Y + HEIGHT as i32)
            .rev()
            .find(|&y| (0..16).any(|dx| (0..16).any(|dz| self.block(cx + dx, y, cz + dz) != Block::Air)))
            .unwrap_or(MIN_Y);
        let built = (0..16).flat_map(|dx| (0..16).map(move |dz| (dx, dz))).map(|(dx, dz)| self.heightmap(cx + dx, cz + dz)).max().unwrap_or(MIN_Y);
        let top = set.max(built);
        self.tops.borrow_mut()[slot] = Some(top);
        Some(top)
    }

    pub fn into_writes(self) -> ChunkWrites {
        ChunkWrites(self.writes, self.routed, self.reach.map(Reach::into_parts))
    }
}
