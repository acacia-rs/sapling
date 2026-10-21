//! The world one source chunk's features see: its own and its neighbours' terrain before features,
//! changed by its own writes only, each write logged per target chunk for replay (see
//! docs/research/worldgen-bds.md, Feature placement).

use std::cell::{OnceCell, RefCell};
use std::sync::Arc;

use super::light;
use super::reach::Reach;
use super::ticks::TickQueue;
pub use super::view_writes::{ChunkWrites, REACH, Routed, Write};
use super::view_writes::{SIDE, slot};
use crate::Dimension;
use crate::block::Block;
use crate::random::Mt19937;
use crate::terrain::{ChunkShape, HEIGHT, MIN_Y};

/// A chunk's pre-worldgen heightmap, None once its decoration has ended.
type Tops<'a> = &'a dyn Fn(i32, i32) -> Option<Arc<[i16; 256]>>;

/// Where heightmaps come from: chunks to scan per column, or their whole light heightmaps.
#[derive(Clone, Copy)]
pub enum Frozen<'a> {
    Chunks(&'a dyn Fn(i32, i32) -> Arc<ChunkShape>),
    Maps(&'a dyn Fn(i32, i32) -> Arc<[i16; 256]>),
}

pub struct View<'a> {
    chunk: (i32, i32),
    load: &'a dyn Fn(i32, i32) -> Arc<ChunkShape>,
    shapes: [OnceCell<Arc<ChunkShape>>; SIDE * SIDE],
    writes: Vec<Vec<Write>>,
    /// Per slot, `highest_block` once asked.
    tops: RefCell<Vec<Option<i32>>>,
    /// The chunks as their last `recomputeHeightMap` saw them.
    frozen: Option<Frozen<'a>>,
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
    /// The Nether's: lava spreads as far as water and three times as often (liquid/flow.rs).
    pub ultrawarm: bool,
    /// The dimension's height range (`BlockSource` +0x3a, +0x38), which BDS answers off the region.
    bounds: (i32, i32),
}

impl<'a> View<'a> {
    pub fn new(chunk: (i32, i32), load: &'a dyn Fn(i32, i32) -> Arc<ChunkShape>) -> Self {
        let slots = SIDE * SIDE;
        Self {
            chunk,
            load,
            shapes: Default::default(),
            writes: vec![Vec::new(); slots],
            tops: RefCell::new(vec![None; slots]),
            frozen: None,
            heights: RefCell::new(vec![None; slots]),
            generated: None,
            ticks: TickQueue::default(),
            routed: Vec::new(),
            reach: None,
            thread_random: None,
            ultrawarm: false,
            bounds: (MIN_Y, MIN_Y + HEIGHT as i32),
        }
    }

    /// The Nether's and the End's own height range, and the Nether's ultrawarm liquids.
    pub fn in_dimension(mut self, dimension: Dimension) -> Self {
        self.ultrawarm = dimension == Dimension::Nether;
        self.bounds = match dimension {
            Dimension::Overworld => self.bounds,
            Dimension::Nether => (0, 128),
            Dimension::End => (0, 256),
        };
        self
    }

    /// Heightmaps come from `frozen`'s chunks, whatever this view writes.
    pub fn with_frozen_heights(mut self, frozen: Frozen<'a>) -> Self {
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
        let shape = self.shapes[slot].get().expect("read just above");
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
        let slot = slot((x >> 4) - self.chunk.0, (z >> 4) - self.chunk.1)?;
        Some((slot, ChunkShape::index((x & 15) as usize, y, (z & 15) as usize)))
    }

    fn read(&self, slot: usize, index: usize) -> Block {
        let (dx, dz) = ((slot / SIDE) as i32 - REACH, (slot % SIDE) as i32 - REACH);
        let chunk = (self.chunk.0 + dx, self.chunk.1 + dz);
        if let Some(reach) = &self.reach {
            reach.read(slot, index, chunk);
        }
        self.shapes[slot].get_or_init(|| (self.load)(chunk.0, chunk.1)).block_at(index)
    }

    /// Air outside the world's height and the writable region.
    pub fn block(&self, x: i32, y: i32, z: i32) -> Block {
        self.locate(x, y, z).map_or(Block::Air, |(slot, index)| self.read(slot, index))
    }

    /// False, writing nothing, outside the world's height and the writable region, and when the
    /// block is already there (`BlockSource::setBlock` 1415f6ae0 reports whether the block changed).
    /// No liquid hears of it (docs/research/worldgen-bds-liquids.md, "Feature writes wake nothing"); for the
    /// fences beside it see view_connect.rs.
    pub(super) fn set_block_alone(&mut self, x: i32, y: i32, z: i32, block: Block) -> bool {
        let Some((slot, index)) = self.locate(x, y, z) else { return false };
        let seen = self.read(slot, index);
        // The liquid layer of a waterlogged plant stays under a waterloggable block.
        let block = if seen == Block::Water || seen.is_waterlogged() { block.into_water() } else { block };
        // A vein or lichen taken out of a water source leaves the water (one-worker run: worldgen-bds-order.md).
        let block = if block == Block::Air && seen.multiface().is_some() && seen.is_waterlogged() { Block::Water } else { block };
        if seen == block {
            return false;
        }
        let shape = self.shapes[slot].get_mut().expect("read just above");
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
            return self.bounds.1;
        }
        let solid = |block: Block| block.material_blocks_motion() && !block.is_leaves();
        (MIN_Y..top).rev().find(|&y| solid(self.block(x, y, z))).map_or(self.bounds.0, |y| y + 1)
    }

    /// `query.heightmap` (lambda 1411a2740) and the chunk heightmap reads alike: the chunk's light
    /// heightmap, one above the highest block that `blocks_light`, at least the world's bottom; outside
    /// the region there is no chunk and the answer is the bottom (worldgen-bds-features.md, "Heightmaps").
    /// Read off the `frozen` chunks when given.
    pub fn heightmap(&self, x: i32, z: i32) -> i32 {
        let Some((slot, _)) = self.locate(x, MIN_Y, z) else { return self.bounds.0 };
        self.region_heightmap(slot, x, z).max(self.bounds.0)
    }

    fn region_heightmap(&self, slot: usize, x: i32, z: i32) -> i32 {
        let top = MIN_Y + HEIGHT as i32;
        let Some(frozen) = self.frozen else {
            return (MIN_Y..top).rev().find(|&y| light::blocks_light(self.block(x, y, z))).map_or(MIN_Y, |y| y + 1);
        };
        let mut maps = self.heights.borrow_mut();
        let (lx, lz) = (x.rem_euclid(16) as usize, z.rem_euclid(16) as usize);
        let (dx, dz) = ((slot / SIDE) as i32 - REACH, (slot % SIDE) as i32 - REACH);
        let chunk = (self.chunk.0 + dx, self.chunk.1 + dz);
        let map = maps[slot].get_or_insert_with(|| match frozen {
            Frozen::Maps(maps) => Box::new(*maps(chunk.0, chunk.1)),
            Frozen::Chunks(_) => Box::new([i16::MIN; 256]),
        });
        let column = &mut map[lz * 16 + lx];
        if let (i16::MIN, Frozen::Chunks(chunks)) = (*column, frozen) {
            *column = light::column(&chunks(chunk.0, chunk.1), lx, lz);
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
