//! The world as one chunk's structure post-process sees it (BDS's `BlockSource` over the 3x3
//! chunks): writes land in the chunk only, reads may reach the neighbours' loaded terrain.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::BoundingBox;
use crate::block::Block;
use crate::feature::{ElementWorld, place_element};
use crate::random::Mt19937;
use crate::terrain::{ChunkShape, MIN_Y};

const TOP: i32 = 319;

/// Spider corridors (start, piece index) that placed their spawner: BDS keeps the flag (+0x5a)
/// on the piece across the chunks it post-processes in.
pub type Spawners = HashSet<((i32, i32), usize)>;

/// What a chunk's structure stage reads past it: the neighbours' carved terrain, and the
/// neighbours BDS post-processed earlier (decoration phases) with their pieces and the
/// spawners they placed.
#[derive(Clone, Copy)]
pub struct Neighbours<'a> {
    pub carved: &'a dyn Fn(i32, i32, i32) -> Block,
    pub placed: &'a dyn Fn(i32, i32, i32) -> Block,
    pub spawners: &'a dyn Fn() -> Spawners,
}

pub struct Canvas<'a> {
    shape: &'a mut ChunkShape,
    chunk: (i32, i32),
    pub bb: BoundingBox,
    neighbour: &'a dyn Fn(i32, i32, i32) -> Block,
    placed: Option<&'a dyn Fn(i32, i32, i32) -> Block>,
    /// Reads past the chunk see `placed` (mineshafts; see stage.rs).
    pub read_placed: bool,
    /// Light heightmap per column, as `LevelChunk` +0xb80 keeps it.
    heights: RefCell<HashMap<(i32, i32), i32>>,
    /// Writes outside the chunk kept (and read back) while a piece placed whole is being drawn.
    spill: Option<(BoundingBox, HashMap<(i32, i32, i32), Block>)>,
    /// The lowest and highest y written inside the chunk.
    pub touched: (i32, i32),
    /// What pieces' features read past the chunk; None places no features.
    features: Option<&'a ElementWorld<'a>>,
    /// Features' writes into the neighbours: (position, block, block replaced).
    pub spilled: Vec<([i32; 3], Block, Block)>,
    /// `setBlock` writes into the neighbours (mineshaft floors), as `spilled`.
    pub unclipped: Vec<([i32; 3], Block, Block)>,
}

impl<'a> Canvas<'a> {
    pub fn new(shape: &'a mut ChunkShape, chunk: (i32, i32), neighbour: &'a dyn Fn(i32, i32, i32) -> Block) -> Self {
        let bb = BoundingBox::chunk(chunk.0, chunk.1);
        Self {
            shape,
            chunk,
            bb,
            neighbour,
            placed: None,
            read_placed: false,
            heights: RefCell::new(HashMap::new()),
            spill: None,
            touched: (i32::MAX, i32::MIN),
            features: None,
            spilled: Vec::new(),
            unclipped: Vec::new(),
        }
    }

    pub fn with_placed(mut self, placed: &'a dyn Fn(i32, i32, i32) -> Block) -> Self {
        self.placed = Some(placed);
        self
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
        for ([x, y, z], block, seen) in writes {
            if self.inside(x, z) {
                self.set(x, y, z, block);
            } else {
                self.spilled.push(([x, y, z], block, seen));
            }
        }
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
        } else if let Some(placed) = self.placed.filter(|_| self.read_placed) {
            placed(x, y, z)
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
            super::trace::write([x, y, z], block);
            self.touched = (self.touched.0.min(y), self.touched.1.max(y));
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

    /// `BlockSource::setBlock` past the chunk box: the chunk's own cells, or a write into a
    /// neighbour that lands where the block read here is still there (structured.rs).
    pub fn set_unclipped(&mut self, x: i32, y: i32, z: i32, block: Block) {
        if self.inside(x, z) {
            self.set(x, y, z, block);
        } else if (MIN_Y..=TOP).contains(&y) {
            let seen = self.get(x, y, z);
            self.unclipped.push(([x, y, z], block, seen));
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

    /// Below the light heightmap (BDS's structure "interior" test); never at the world's top.
    pub fn interior(&self, x: i32, y: i32, z: i32) -> bool {
        y < TOP + 1 && (y < MIN_Y || self.height(x, z) > y)
    }
}

fn counts_for_height(block: Block) -> bool {
    block != Block::Air && !block.is_plant()
}
