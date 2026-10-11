//! The structure stage across chunks (`structurePostProcessChunk`, after carvers and before
//! decoration). A chunk's pieces write inside it only, but the features some pieces place
//! (village trees, piles, sculk patches) write through the 3x3 block source: they see the
//! neighbours' carved terrain, and land in a neighbour after its own pieces, where the block
//! they saw is still there. (Ordering neighbours by the decoration phases, features seeing
//! earlier neighbours with their pieces, matched the oracle worse: ancient city +204 cells.)

pub(crate) mod replay;

use std::sync::Arc;

use crate::block::Block;
use crate::feature;
use crate::structure::trace;
use crate::terrain::ChunkShape;
use crate::{OverworldGenerator, structure};

/// A feature's write past its chunk: position, block, block it replaced.
type Spill = ([i32; 3], Block, Block);

/// A chunk after its own pieces.
pub struct Pieced {
    shape: Arc<ChunkShape>,
    /// Its pieces' features' writes into the neighbours.
    spilled: Vec<Spill>,
    /// Its mineshaft floors' writes into the neighbours.
    unclipped: Vec<Spill>,
    spawners: structure::Spawners,
    /// The heights it changed, for `rejoin_connections`.
    touched: Option<(i32, i32)>,
}

impl Pieced {
    /// A chunk no piece wrote.
    fn bare(shape: Arc<ChunkShape>) -> Self {
        Self { shape, spilled: Vec::new(), unclipped: Vec::new(), spawners: Default::default(), touched: None }
    }

    /// A chunk as a replayed run left it: its neighbours' mineshaft floors have landed.
    fn replayed(shape: ChunkShape, touched: Option<(i32, i32)>, spilled: Vec<Spill>) -> Self {
        Self { touched, spilled, ..Self::bare(Arc::new(shape)) }
    }
}

fn widen(heights: Option<(i32, i32)>, y: i32) -> Option<(i32, i32)> {
    Some(heights.map_or((y, y), |(lo, hi)| (lo.min(y), hi.max(y))))
}

impl OverworldGenerator {
    /// `shape`, the chunk after its carvers, becomes the chunk after the structure stage.
    pub fn place_structures(&self, shape: &mut ChunkShape, chunk_x: i32, chunk_z: i32) {
        let chunk = (chunk_x, chunk_z);
        let pieced = self.pieced(chunk);
        *shape = (*pieced.shape).clone();
        let mut heights = pieced.touched;
        let mut land = |spills: &[Spill]| {
            for &([x, y, z], block, seen) in spills.iter().filter(|(p, ..)| (p[0] >> 4, p[2] >> 4) == chunk) {
                let (lx, lz) = ((x & 15) as usize, (z & 15) as usize);
                if shape.block(lx, y, lz) == seen {
                    shape.set_block(lx, y, lz, block);
                    trace::write([x, y, z], block);
                    heights = widen(heights, y);
                }
            }
        };
        for source in self.structures.feature_sources(&self.density, chunk) {
            let spilled = &self.pieced(source).spilled;
            trace::label(|| format!("feature spill from {source:?}"));
            land(spilled);
        }
        for source in self.structures.later_mineshaft_writers(&self.density, chunk) {
            let unclipped = &self.pieced(source).unclipped;
            trace::label(|| format!("mineshaft floor from {source:?}"));
            land(unclipped);
        }
        if let Some(heights) = heights {
            let neighbour = |x: i32, y: i32, z: i32| self.pieced((x >> 4, z >> 4)).shape.block((x & 15) as usize, y, (z & 15) as usize);
            trace::label(|| "rejoin".into());
            structure::rejoin_connections(shape, chunk, heights, &neighbour);
        }
    }

    /// The chunk's structure-stage writes in order with what wrote each (structure/trace.rs), for probes.
    #[doc(hidden)]
    pub fn structure_trace(&self, chunk_x: i32, chunk_z: i32) -> Vec<(([i32; 3], Block), String)> {
        let inside = |p: &[i32; 3]| (p[0] >> 4, p[2] >> 4) == (chunk_x, chunk_z);
        // Warm the caches so that neighbours' pieces stay out of the trace where they can.
        self.loaded(chunk_x, chunk_z);
        let (_, writes) = trace::capture(|| {
            let mut shape = self.pieced_uncached(chunk_x, chunk_z);
            self.place_structures(&mut shape, chunk_x, chunk_z);
        });
        writes.into_iter().filter(|(p, ..)| inside(p)).map(|(p, block, label)| ((p, block), label.to_string())).collect()
    }

    /// The chunk's pieces placed afresh (no cache, no joins), for probes that change them.
    #[doc(hidden)]
    pub fn pieced_uncached(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        let mut shape = (*self.carved(chunk_x, chunk_z)).clone();
        self.place_pieces(&mut shape, (chunk_x, chunk_z));
        shape
    }

    pub(crate) fn pieced(&self, chunk: (i32, i32)) -> Arc<Pieced> {
        if let Some(replay) = &self.structure_replay {
            return self.replayed(replay, chunk);
        }
        self.pieced.get(chunk, || {
            let mut shape = (*self.carved(chunk.0, chunk.1)).clone();
            let placed = self.place_pieces(&mut shape, chunk);
            let touched = placed.as_ref().map(|p| p.touched);
            let (spilled, unclipped, spawners) = placed.map_or_else(Default::default, |p| (p.spilled, p.unclipped, p.spawners));
            Pieced { shape: Arc::new(shape), spilled, unclipped, spawners, touched }
        })
    }

    /// The chunk's pieces, reading past it through the neighbours' carved terrain.
    fn place_pieces(&self, shape: &mut ChunkShape, chunk: (i32, i32)) -> Option<structure::Placed> {
        let earlier = |n: (i32, i32)| feature::phase(n) < feature::phase(chunk);
        let carved = |x: i32, y: i32, z: i32| self.carved(x >> 4, z >> 4).block((x & 15) as usize, y, (z & 15) as usize);
        let placed = |x: i32, y: i32, z: i32| {
            let n = (x >> 4, z >> 4);
            if earlier(n) { self.pieced(n).shape.block((x & 15) as usize, y, (z & 15) as usize) } else { carved(x, y, z) }
        };
        let spawners = || {
            let around = (chunk.0 - 1..=chunk.0 + 1).flat_map(|x| (chunk.1 - 1..=chunk.1 + 1).map(move |z| (x, z)));
            around.filter(|&n| earlier(n)).flat_map(|n| self.pieced(n).spawners.clone()).collect()
        };
        let generated_tops = || self.generated_tops(chunk.0, chunk.1);
        let neighbours = structure::Neighbours { carved: &carved, placed: &placed, spawners: &spawners, generated_tops: &generated_tops, shaft_order: None };
        let biomes = self.chunk_biomes(chunk.0, chunk.1);
        let load = |x: i32, z: i32| self.carved(x, z);
        let biome = |[x, y, z]: [i32; 3]| Some(self.chunk_biomes(x >> 4, z >> 4).biome((x & 15) as usize, y, (z & 15) as usize));
        let surface = |qx: i32, qz: i32| self.density.preliminary_surface(qx, qz);
        let features = feature::ElementWorld { load: &load, biome: &biome, surface: &surface };
        self.structures.place(&self.density, shape, chunk, neighbours, &biomes, &features)
    }
}
