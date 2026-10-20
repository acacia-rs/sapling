//! Probes of the structure stage: post-processes run again, outside the caches, to trace or change them.

use super::Stage;
use crate::block::Block;
use crate::random::{LargeFeatureSeed, Mt19937};
use crate::structure::trace;
use crate::terrain::ChunkShape;
use crate::{OverworldGenerator, feature};

impl OverworldGenerator {
    /// The chunk's structure-stage writes in order with what wrote each (structure/trace.rs).
    #[doc(hidden)]
    pub fn structure_trace(&self, chunk_x: i32, chunk_z: i32) -> Vec<(([i32; 3], Block), String)> {
        let chunk = (chunk_x, chunk_z);
        let inside = |p: &[i32; 3]| (p[0] >> 4, p[2] >> 4) == chunk;
        // The caches hold everything but the post-processes traced below and the rejoin.
        self.loaded(chunk_x, chunk_z);
        let (_, writes) = trace::capture(|| {
            for (_, writer) in self.writers(chunk) {
                self.post_process(writer);
            }
            let mut shape = (*self.carved(chunk_x, chunk_z)).clone();
            self.place_structures(&mut shape, chunk_x, chunk_z);
        });
        writes.into_iter().filter(|(p, ..)| inside(p)).map(|(p, block, label)| ((p, block), label.to_string())).collect()
    }

    /// A structure feature placed alone from a chunk over its carved terrain, its structure
    /// generator `skip` draws in: (position, block, block replaced).
    #[doc(hidden)]
    pub fn structure_feature_writes(&self, name: &str, chunk: (i32, i32), at: [i32; 3], skip: usize) -> Vec<([i32; 3], Block, Block)> {
        let mut random = Mt19937::new(LargeFeatureSeed::new(self.seed as u32).chunk(chunk.0, chunk.1));
        (0..skip).for_each(|_| _ = random.next_u32());
        let load = |x: i32, z: i32| self.carved(x, z);
        let biome = |[x, y, z]: [i32; 3]| Some(self.chunk_biomes(x >> 4, z >> 4).biome((x & 15) as usize, y, (z & 15) as usize));
        let surface = |qx: i32, qz: i32| self.density.preliminary_surface(qx, qz);
        let world = feature::ElementWorld { load: &load, biome: &biome, surface: &surface };
        feature::place_element(name, chunk, self.carved(chunk.0, chunk.1), &world, &mut random, at).unwrap_or_default()
    }

    /// The chunk after its own post-process, run afresh (no joins).
    #[doc(hidden)]
    pub fn pieced_uncached(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        self.warm_structures();
        let chunk = (chunk_x, chunk_z);
        match self.post_process(chunk) {
            Some(source) => (*source.own.shape).clone(),
            None => (*self.staged_before(chunk, self.structure_rank(chunk).unwrap_or(0)).shape).clone(),
        }
    }
}
