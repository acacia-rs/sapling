//! The Nether's structure stage: the shared engine (structured.rs) over its carved chunks, each
//! post-process placing the Nether's starts in registry order (structure/nether.rs).

use std::sync::Arc;

use super::NetherGenerator;
use crate::cache::SourceCache;
use crate::chunk::ChunkShape;
use crate::structured::order::Rank;
use crate::structured::replay::{self, StructureStep};
use crate::structured::{Around, Pieced, Source, Stage};

impl NetherGenerator {
    /// Runs the structure stage as one BDS run did (`OverworldGenerator::set_structure_order`).
    #[doc(hidden)]
    pub fn set_structure_order(&mut self, steps: &[StructureStep]) {
        let (order, caches) = replay::logged_order::<ChunkShape, ()>(steps);
        (self.carved, self.pieced, self.staged) = (caches.carved, caches.pieced, caches.staged);
        self.structure_order = order;
    }

    /// The carved chunk, then the structure stage: what decoration starts from.
    pub fn loaded_blocks(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        (*self.loaded(chunk_x, chunk_z)).clone()
    }

    pub(super) fn loaded(&self, chunk_x: i32, chunk_z: i32) -> Arc<ChunkShape> {
        self.loaded.get((chunk_x, chunk_z), || {
            let mut shape = (*self.carved(chunk_x, chunk_z)).clone();
            Stage::place_structures(self, &mut shape, chunk_x, chunk_z);
            shape.share();
            shape
        })
    }

    /// The chunk BDS's `/locate structure fortress` answers from a block position.
    pub fn locate_fortress(&self, x: i32, z: i32) -> Option<(i32, i32)> {
        let quart = |qx: i32, qz: i32| self.quart_biome(qx, qz);
        self.structures.locate(crate::structure::nether::SharedKind::Fortress, (x, z), &quart)
    }
}

impl Stage for NetherGenerator {
    fn stage_carved(&self, chunk: (i32, i32)) -> Arc<ChunkShape> {
        self.carved(chunk.0, chunk.1)
    }

    fn structure_rank(&self, chunk: (i32, i32)) -> Option<Rank> {
        self.structure_order.rank(chunk)
    }

    fn pieced_cache(&self) -> &SourceCache<(i32, i32), Option<Source>> {
        &self.pieced
    }

    fn staged_cache(&self) -> &SourceCache<(i32, i32, u8), Pieced> {
        &self.staged
    }

    fn warm_structures(&self) {
        replay::warm_logged(self, &self.structure_order);
    }

    fn post_process(&self, chunk: (i32, i32)) -> Option<Source> {
        let rank = self.structure_rank(chunk)?;
        let quart = |qx: i32, qz: i32| self.quart_biome(qx, qz);
        let reaching = self.structures.reaching(chunk, &quart)?;
        let before = |n: (i32, i32)| self.staged_before(n, rank);
        let around = Around::new(chunk, &before);
        let own = around.get(chunk).clone();
        let mut shape = (**own.shape()).clone();
        let block = |x: i32, y: i32, z: i32| around.get((x >> 4, z >> 4)).shape().block((x & 15) as usize, y, (z & 15) as usize);
        let placed = self.structures.place(&reaching, &mut shape, chunk, &block);
        Some(Source::new(chunk, &own, shape, placed))
    }
}
