//! The structure stage across chunks (`structurePostProcessChunk`, after carvers and before
//! decoration). A chunk's pieces write inside it only, but the features some pieces place
//! (village trees, piles, sculk patches) write through the 3x3 block source: they see the
//! neighbours' carved terrain, and land in a neighbour after its own pieces, where the block
//! they saw is still there. (Ordering neighbours by the decoration phases, features seeing
//! earlier neighbours with their pieces, matched the oracle worse: ancient city +204 cells.)

use std::sync::Arc;

use crate::block::Block;
use crate::feature;
use crate::terrain::ChunkShape;
use crate::{OverworldGenerator, structure};

/// A feature's write past its chunk: position, block, block it replaced.
type Spill = ([i32; 3], Block, Block);

/// A chunk after its own pieces.
pub struct Pieced {
    shape: Arc<ChunkShape>,
    /// Its pieces' features' writes into the neighbours.
    spilled: Vec<Spill>,
    /// The heights it changed, for `rejoin_connections`.
    touched: Option<(i32, i32)>,
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
        for source in self.structures.feature_sources(&self.density, chunk) {
            for &([x, y, z], block, seen) in self.pieced(source).spilled.iter().filter(|(p, ..)| (p[0] >> 4, p[2] >> 4) == chunk) {
                let (lx, lz) = ((x & 15) as usize, (z & 15) as usize);
                if shape.block(lx, y, lz) == seen {
                    shape.set_block(lx, y, lz, block);
                    heights = widen(heights, y);
                }
            }
        }
        if let Some(heights) = heights {
            let neighbour = |x: i32, y: i32, z: i32| self.pieced((x >> 4, z >> 4)).shape.block((x & 15) as usize, y, (z & 15) as usize);
            structure::rejoin_connections(shape, chunk, heights, &neighbour);
        }
    }

    /// The chunk's pieces placed afresh (no cache, no joins), for probes that change them.
    #[doc(hidden)]
    pub fn pieced_uncached(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        let mut shape = (*self.carved(chunk_x, chunk_z)).clone();
        self.place_pieces(&mut shape, (chunk_x, chunk_z));
        shape
    }

    pub(crate) fn pieced(&self, chunk: (i32, i32)) -> Arc<Pieced> {
        self.pieced.get(chunk, || {
            let mut shape = (*self.carved(chunk.0, chunk.1)).clone();
            let placed = self.place_pieces(&mut shape, chunk);
            let touched = placed.as_ref().map(|p| p.touched);
            Pieced { shape: Arc::new(shape), spilled: placed.map_or_else(Vec::new, |p| p.spilled), touched }
        })
    }

    /// The chunk's pieces, reading past it through the neighbours' carved terrain.
    fn place_pieces(&self, shape: &mut ChunkShape, chunk: (i32, i32)) -> Option<structure::Placed> {
        let carved = |x: i32, y: i32, z: i32| self.carved(x >> 4, z >> 4).block((x & 15) as usize, y, (z & 15) as usize);
        let placed = |x: i32, y: i32, z: i32| {
            let n = (x >> 4, z >> 4);
            if feature::phase(n) < feature::phase(chunk) { self.pieced(n).shape.block((x & 15) as usize, y, (z & 15) as usize) } else { carved(x, y, z) }
        };
        let neighbours = structure::Neighbours { carved: &carved, placed: &placed };
        let biomes = self.chunk_biomes(chunk.0, chunk.1);
        let load = |x: i32, z: i32| self.carved(x, z);
        let biome = |[x, y, z]: [i32; 3]| Some(self.chunk_biomes(x >> 4, z >> 4).biome((x & 15) as usize, y, (z & 15) as usize));
        let surface = |qx: i32, qz: i32| self.density.preliminary_surface(qx, qz);
        let features = feature::ElementWorld { load: &load, biome: &biome, surface: &surface };
        self.structures.place(&self.density, shape, chunk, neighbours, &biomes, &features)
    }
}
