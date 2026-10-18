//! The End's structure stage: the shared engine (structured.rs) over its terrain chunks, each post-process placing
//! the cities that reach it (structure/end.rs).

use std::sync::Arc;

use super::EndGenerator;
use crate::cache::SourceCache;
use crate::chunk::ChunkShape;
use crate::structure::end::StartInputs;
use crate::structured::order::Rank;
use crate::structured::replay::{self, StructureStep};
use crate::structured::{Around, Pieced, Source, Stage};

impl EndGenerator {
    /// Runs the structure stage as one BDS run did (`OverworldGenerator::set_structure_order`).
    #[doc(hidden)]
    pub fn set_structure_order(&mut self, steps: &[StructureStep]) {
        let (order, caches) = replay::logged_order::<ChunkShape, ()>(steps);
        (self.terrain, self.pieced, self.staged) = (caches.carved, caches.pieced, caches.staged);
        self.structure_order = order;
    }

    /// The terrain, then the structure stage: what decoration starts from.
    pub fn loaded_blocks(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        (*self.loaded(chunk_x, chunk_z)).clone()
    }

    pub(super) fn loaded(&self, chunk_x: i32, chunk_z: i32) -> Arc<ChunkShape> {
        self.loaded.get((chunk_x, chunk_z), || {
            let mut shape = (*self.terrain(chunk_x, chunk_z)).clone();
            Stage::place_structures(self, &mut shape, chunk_x, chunk_z);
            shape.share();
            shape
        })
    }

    fn start_inputs<R>(&self, then: impl FnOnce(&StartInputs) -> R) -> R {
        let terrain = |x, z| self.terrain(x, z);
        let island = |x, z| self.island_height(x, z, 1, 1);
        then(&StartInputs { terrain: &terrain, island: &island })
    }

    /// The chunk BDS's `/locate structure end_city` answers from a block position (`findNearestFeaturePositionBySpacing`
    /// 141efb740: region rings out to 100, the first start found).
    pub fn locate_city(&self, x: i32, z: i32) -> Option<(i32, i32)> {
        let (cx, cz) = (x >> 4, z >> 4);
        let spacing = crate::structure::end::GRID.spacing;
        self.start_inputs(|inputs| {
            for ring in 0..=100i32 {
                for dx in -ring..=ring {
                    for dz in (-ring..=ring).filter(|dz: &i32| dx.abs() == ring || dz.abs() == ring) {
                        let region = ((dx * spacing + cx).div_euclid(spacing) * spacing, (dz * spacing + cz).div_euclid(spacing) * spacing);
                        let start = crate::structure::end::GRID.candidate(self.seed, region);
                        if self.structures.city(start, inputs).is_some() {
                            return Some(start);
                        }
                    }
                }
            }
            None
        })
    }

    /// The start chunks of the cities whose pieces meet `chunk`.
    pub fn cities_reaching(&self, chunk_x: i32, chunk_z: i32) -> Vec<(i32, i32)> {
        self.start_inputs(|inputs| self.structures.reaching((chunk_x, chunk_z), inputs)).into_iter().flatten().filter_map(|c| c.as_ref().as_ref().map(|c| c.start)).collect()
    }
}

impl Stage for EndGenerator {
    fn stage_carved(&self, chunk: (i32, i32)) -> Arc<ChunkShape> {
        self.terrain(chunk.0, chunk.1)
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
        let cities = self.start_inputs(|inputs| self.structures.reaching(chunk, inputs))?;
        let before = |n: (i32, i32)| self.staged_before(n, rank);
        let around = Around::new(chunk, &before);
        let own = around.get(chunk).clone();
        let mut shape = (**own.shape()).clone();
        let block = |x: i32, y: i32, z: i32| around.get((x >> 4, z >> 4)).shape().block((x & 15) as usize, y, (z & 15) as usize);
        let placed = self.structures.place(&cities, &mut shape, chunk, &block);
        Some(Source::new(chunk, &own, shape, placed))
    }
}
