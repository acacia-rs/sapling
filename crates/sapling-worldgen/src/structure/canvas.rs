//! The world as one chunk's structure post-process sees it (BDS's `BlockSource` over the 3x3
//! chunks): the chunk itself, the eight around it as the post-processes before this one left
//! them, nothing further out.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::BoundingBox;
use super::template::Record;
use crate::block::Block;
use crate::feature::{ElementWorld, place_element};
use crate::random::Mt19937;
use crate::terrain::{ChunkShape, MIN_Y};

const TOP: i32 = 319;

/// Spider corridors (start, piece index) that placed their spawner: BDS keeps the flag (+0x5a)
/// on the piece across the chunks it post-processes in.
pub type Spawners = HashSet<((i32, i32), usize)>;

/// Cells whose neighbour-shaped states a template pass decided (template/update.rs).
pub type Settled = HashSet<(i32, i32, i32)>;

/// A write into one of the eight chunks around the one post-processed.
pub type Past = ([i32; 3], Block);

/// What a chunk's structure post-process takes from the ones before it
/// (docs/research/worldgen-bds-order.md, "One engine").
#[derive(Clone, Copy)]
pub struct Neighbours<'a> {
    /// A block of the eight chunks around.
    pub placed: &'a dyn Fn(i32, i32, i32) -> Block,
    /// The spider corridors whose spawner is placed.
    pub spawners: &'a dyn Fn() -> Spawners,
    /// The chunk's own pre-worldgen heightmap (`LevelChunk` +0xf80: the terrain before its surface and carvers).
    pub generated_tops: &'a dyn Fn() -> Arc<[i16; 256]>,
    /// Mineshaft starts in the order BDS's start map lists them (structure/start_map.rs).
    pub shaft_order: &'a dyn Fn() -> Arc<[(i32, i32)]>,
    /// The height the desert pyramid of a start chunk took in an earlier chunk.
    pub temple_height: &'a dyn Fn((i32, i32)) -> Option<i32>,
    /// The canonical order's exception: a pyramid no chunk has placed yet takes its height from its
    /// start chunk's columns, as if that chunk ran first of its four.
    pub pyramid_from_start: bool,
}

pub struct Canvas<'a> {
    shape: &'a mut ChunkShape,
    chunk: (i32, i32),
    pub bb: BoundingBox,
    neighbour: &'a dyn Fn(i32, i32, i32) -> Block,
    /// The piece being placed is written whole by its start chunk (igloos, shipwrecks, ruined
    /// portals): its blocks past the chunk land in the 3x3 too.
    pub whole: bool,
    /// What template placements file for their update pass.
    pub record: Record,
    /// Light heightmap per column, as `LevelChunk` +0xb80 keeps it.
    heights: RefCell<HashMap<(i32, i32), i32>>,
    /// The lowest and highest y written inside the chunk.
    pub touched: (i32, i32),
    /// What pieces' features read past the chunk; None places no features.
    features: Option<&'a ElementWorld<'a>>,
    /// Features' writes into the neighbours.
    pub spilled: Vec<Past>,
    /// `setBlock` writes into the neighbours: mineshaft floors, whole pieces, update passes.
    pub unclipped: Vec<Past>,
    /// `unclipped` as this post-process reads it back.
    written: HashMap<(i32, i32, i32), Block>,
    /// Fences this post-process's mineshaft pieces placed: still the piece's unjoined fence block
    /// to BDS, which `canBeReplaced` keeps (a stronghold library's railing is not).
    pub shaft_fences: HashSet<(i32, i32, i32)>,
}

impl<'a> Canvas<'a> {
    pub fn new(shape: &'a mut ChunkShape, chunk: (i32, i32), neighbour: &'a dyn Fn(i32, i32, i32) -> Block) -> Self {
        let bb = BoundingBox::chunk(chunk.0, chunk.1);
        Self {
            shape,
            chunk,
            bb,
            neighbour,
            whole: false,
            record: Record::default(),
            heights: RefCell::new(HashMap::new()),
            touched: (i32::MAX, i32::MIN),
            features: None,
            spilled: Vec::new(),
            unclipped: Vec::new(),
            written: HashMap::new(),
            shaft_fences: HashSet::new(),
        }
    }

    pub fn with_features(mut self, world: &'a ElementWorld<'a>) -> Self {
        self.features = Some(world);
        self
    }

    /// A piece's feature at `at`, drawing from `random`, through the 3x3 block source: writes in
    /// the chunk land, the others go to `spilled`.
    pub fn place_feature(&mut self, name: &str, random: &mut Mt19937, at: [i32; 3]) {
        let Some(world) = self.features else { return };
        let centre = Arc::new(self.shape.clone());
        let Some(writes) = place_element(name, self.chunk, centre, world, random, at) else { return };
        for ([x, y, z], block, _) in writes {
            if self.inside(x, z) {
                self.set(x, y, z, block);
            } else {
                self.spilled.push(([x, y, z], block));
                super::trace::write([x, y, z], block);
            }
        }
    }

    fn inside(&self, x: i32, z: i32) -> bool {
        x >> 4 == self.chunk.0 && z >> 4 == self.chunk.1
    }

    /// Whether the post-process's `BlockSource` holds the column's chunk; a piece reaches further.
    fn in_view(&self, x: i32, z: i32) -> bool {
        ((x >> 4) - self.chunk.0).abs() <= 1 && ((z >> 4) - self.chunk.1).abs() <= 1
    }

    /// Any block: the chunk's own, a neighbour's, air outside the world's heights and the view.
    pub fn get(&self, x: i32, y: i32, z: i32) -> Block {
        if !(MIN_Y..=TOP).contains(&y) || !self.in_view(x, z) {
            Block::Air
        } else if self.inside(x, z) {
            self.shape.block((x & 15) as usize, y, (z & 15) as usize)
        } else if let Some(&block) = self.written.get(&(x, y, z)) {
            block
        } else {
            (self.neighbour)(x, y, z)
        }
    }

    /// A block inside the chunk; air elsewhere (BDS's pieces read through the chunk box).
    pub fn get_clipped(&self, x: i32, y: i32, z: i32) -> Block {
        if self.bb.contains(x, y, z) { self.get(x, y, z) } else { Block::Air }
    }

    /// Writes inside the chunk, and anywhere in the view while a `whole` piece is placed.
    pub fn set(&mut self, x: i32, y: i32, z: i32, block: Block) {
        if self.bb.contains(x, y, z) {
            let before = self.shape.block((x & 15) as usize, y, (z & 15) as usize);
            // A block that can hold water keeps the water it is set into (an igloo cellar's bars in a flooded cave).
            let block = if before == Block::Water { super::template::soaked(block) } else { block };
            self.record.note((x, y, z), before, block);
            self.shape.set_block((x & 15) as usize, y, (z & 15) as usize, block);
            super::trace::write([x, y, z], block);
            self.touched = (self.touched.0.min(y), self.touched.1.max(y));
            let mut heights = self.heights.borrow_mut();
            if let Some(h) = heights.get_mut(&(x, z))
                && counts_for_height(block)
            {
                *h = (*h).max(y + 1);
            }
        } else if self.whole && (MIN_Y..=TOP).contains(&y) && self.in_view(x, z) {
            self.record.note((x, y, z), self.get(x, y, z), block);
            self.write_past(x, y, z, block);
        }
    }

    fn write_past(&mut self, x: i32, y: i32, z: i32, block: Block) {
        self.unclipped.push(([x, y, z], block));
        self.written.insert((x, y, z), block);
        super::trace::write([x, y, z], block);
    }

    /// An update pass's write (template/update.rs): anywhere in the view.
    pub fn set_in_view(&mut self, x: i32, y: i32, z: i32, block: Block) {
        if self.bb.contains(x, y, z) {
            self.shape.set_block((x & 15) as usize, y, (z & 15) as usize, block);
            super::trace::write([x, y, z], block);
            self.touched = (self.touched.0.min(y), self.touched.1.max(y));
        } else if (MIN_Y..=TOP).contains(&y) && self.in_view(x, z) {
            self.write_past(x, y, z, block);
        } else {
            return;
        }
        self.record.settle((x, y, z));
    }

    /// `BlockSource::setBlock` past the chunk box: anywhere in the view.
    pub fn set_unclipped(&mut self, x: i32, y: i32, z: i32, block: Block) {
        if self.inside(x, z) {
            self.set(x, y, z, block);
        } else if (MIN_Y..=TOP).contains(&y) && self.in_view(x, z) {
            self.write_past(x, y, z, block);
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

    /// One above the highest solid block, or fluid too when `fluids`.
    pub fn top(&self, x: i32, z: i32, fluids: bool) -> i32 {
        (MIN_Y..=TOP).rev().find(|&y| self.get(x, y, z).is_solid() || fluids && self.get(x, y, z).is_fluid()).map_or(MIN_Y, |y| y + 1)
    }

    /// Below the light heightmap (BDS's structure "interior" test); never at the world's top, nor
    /// in a column whose chunk the view lacks (`getChunk` null).
    pub fn interior(&self, x: i32, y: i32, z: i32) -> bool {
        y < TOP + 1 && (y < MIN_Y || self.in_view(x, z) && self.height(x, z) > y)
    }
}

fn counts_for_height(block: Block) -> bool {
    block != Block::Air && !block.is_plant()
}
