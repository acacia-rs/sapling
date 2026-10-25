//! The overworld's post-process: `Structures::place` over the 3x3 the earlier post-processes left.

use std::sync::Arc;

use super::order::{Order, Rank};
use super::{Around, Pieced, Source, Stage};
use crate::cache::SourceCache;
use crate::feature;
use crate::structure::{self, Spawners};
use crate::terrain::ChunkShape;
use crate::OverworldGenerator;

impl OverworldGenerator {
    /// `shape`, the chunk after its carvers, becomes the chunk after the structure stage.
    pub fn place_structures(&self, shape: &mut ChunkShape, chunk_x: i32, chunk_z: i32) {
        Stage::place_structures(self, shape, chunk_x, chunk_z);
    }
}

impl Stage for OverworldGenerator {
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
        self.warm_logged();
    }

    fn post_process(&self, chunk: (i32, i32)) -> Option<Source> {
        let rank = self.structure_rank(chunk)?;
        let reaching = self.structures.reaching(&self.density, chunk)?;
        Some(self.post_process_in(self, chunk, rank, &reaching, &|| self.generated_tops(chunk.0, chunk.1)))
    }
}

impl OverworldGenerator {
    /// `reaching`'s post-process of `chunk`, ranked `rank` among `stage`'s, over the chunks as
    /// `stage`'s earlier post-processes left them; `tops` is the chunk's pre-worldgen heightmap.
    pub(super) fn post_process_in(&self, stage: &impl Stage, chunk: (i32, i32), rank: Rank, reaching: &structure::Reaching, tops: &dyn Fn() -> Arc<[i16; 256]>) -> Source {
        let before = |n: (i32, i32)| stage.staged_before(n, rank);
        let around = Around::new(chunk, &before);
        let own = around.get(chunk).clone();
        let mut shape = (*own.shape).clone();
        let block = |x: i32, y: i32, z: i32| around.get((x >> 4, z >> 4)).shape.block((x & 15) as usize, y, (z & 15) as usize);
        // A corridor is at most three chunks long.
        let spawners = || stage.earlier_sources(chunk, rank, 2).iter().flat_map(|source| (**source).as_ref().map_or_else(Spawners::new, |s| s.spawners().clone())).collect();
        let shaft_order = |starts: &mut [(i32, i32)]| self.shaft_order(chunk, starts);
        let temple_height = |start: (i32, i32)| stage.earlier_sources(chunk, rank, 1).iter().find_map(|source| (**source).as_ref()?.temple_height(start));
        let neighbours = structure::Neighbours { placed: &block, spawners: &spawners, generated_tops: tops, shaft_order: &shaft_order, temple_height: &temple_height, pyramid_from_start: matches!(self.structure_order, Order::Canonical) };
        // Pieces' features read the same state; a feature's region is the 3x3.
        let load = |x: i32, z: i32| if around.holds((x, z)) { around.get((x, z)).shape.clone() } else { stage.stage_carved((x, z)) };
        let biome = |[x, y, z]: [i32; 3]| Some(self.chunk_biomes(x >> 4, z >> 4).biome((x & 15) as usize, y, (z & 15) as usize));
        let surface = |qx: i32, qz: i32| self.density.preliminary_surface(qx, qz);
        let features = feature::ElementWorld { load: &load, biome: &biome, surface: &surface };
        let biomes = self.chunk_biomes(chunk.0, chunk.1);
        let placed = self.structures.place(&self.density, reaching, &mut shape, chunk, neighbours, &biomes, &features);
        Source::new(chunk, &own, shape, placed)
    }
}
