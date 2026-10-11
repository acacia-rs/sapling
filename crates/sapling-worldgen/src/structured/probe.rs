//! Probes of the structure stage: post-processes run again, outside the caches, to trace or change them.

use crate::block::Block;
use crate::structure::trace;
use crate::terrain::ChunkShape;
use crate::OverworldGenerator;

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
