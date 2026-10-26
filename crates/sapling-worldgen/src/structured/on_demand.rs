//! A structure placed on demand: its forced start (structure/on_demand.rs) post-processed in
//! every chunk it reaches, in the canonical order, over the chunks a caller loads instead of the
//! generator's, then compared with them.

use std::sync::Arc;

use super::order::{Order, Rank};
use super::{Pieced, Source, Stage};
use crate::block::Block;
use crate::cache::SourceCache;
use crate::structure::{BoundingBox, Forced, PlaceFailure, StructureKind};
use crate::terrain::{ChunkShape, HEIGHT, MIN_Y};
use crate::OverworldGenerator;

type Load<'a> = &'a dyn Fn(i32, i32) -> Arc<ChunkShape>;

/// Entries kept per cache; more only recompute.
const LIMIT: usize = 4096;

struct OnDemand<'a> {
    generator: &'a OverworldGenerator,
    forced: Forced,
    /// The chunks post-processed (x0, z0, x1, z1).
    chunks: (i32, i32, i32, i32),
    load: Load<'a>,
    loaded: SourceCache<(i32, i32), Arc<ChunkShape>>,
    pieced: SourceCache<(i32, i32), Option<Source>>,
    staged: SourceCache<(i32, i32, u8), Pieced>,
}

impl Stage for OnDemand<'_> {
    fn stage_carved(&self, chunk: (i32, i32)) -> Arc<ChunkShape> {
        (*self.loaded.get(chunk, || (self.load)(chunk.0, chunk.1))).clone()
    }

    fn structure_rank(&self, (x, z): (i32, i32)) -> Option<Rank> {
        let (x0, z0, x1, z1) = self.chunks;
        ((x0..=x1).contains(&x) && (z0..=z1).contains(&z)).then(|| Order::Canonical.rank((x, z))).flatten()
    }

    fn pieced_cache(&self) -> &SourceCache<(i32, i32), Option<Source>> {
        &self.pieced
    }

    fn staged_cache(&self) -> &SourceCache<(i32, i32, u8), Pieced> {
        &self.staged
    }

    fn warm_structures(&self) {}

    fn post_process(&self, chunk: (i32, i32)) -> Option<Source> {
        let rank = self.structure_rank(chunk)?;
        // TODO: the loaded chunk's own heights stand in for its pre-worldgen heightmap (villages' terrain matching).
        let tops = || Arc::new(self.stage_carved(chunk).tops());
        Some(self.generator.post_process_in(self, chunk, rank, &self.forced.reaching, &tops))
    }
}

impl OnDemand<'_> {
    /// Every block the post-processes changed, by chunk, after `rejoin_connections`.
    fn writes(&self) -> Vec<([i32; 3], Block)> {
        let (x0, z0, x1, z1) = self.chunks;
        let mut writes = Vec::new();
        for chunk in (x0 - 1..=x1 + 1).flat_map(|x| (z0 - 1..=z1 + 1).map(move |z| (x, z))) {
            if self.staged_after(chunk).touched.is_none() {
                continue;
            }
            let before = self.stage_carved(chunk);
            let mut after = ChunkShape::empty();
            self.place_structures(&mut after, chunk.0, chunk.1);
            for y in MIN_Y..MIN_Y + HEIGHT as i32 {
                for (z, x) in (0..16).flat_map(|z| (0..16).map(move |x| (z, x))) {
                    let block = after.block(x, y, z);
                    if block != before.block(x, y, z) {
                        writes.push(([chunk.0 * 16 + x as i32, y, chunk.1 * 16 + z as i32], block));
                    }
                }
            }
        }
        writes
    }
}

impl OverworldGenerator {
    /// The blocks the structure `kind` places when it starts in chunk `start` (as if the placement
    /// grid put a start there; biome filters are skipped too, though a biome still picks the
    /// variant), drawn over the chunk shapes `load` gives for every chunk its pieces cover: world
    /// position and block, every chunk's writes. Err(Unsupported) for kinds this cannot run on
    /// demand; Err(Failed) when the start's own checks fail there (no room, wrong height...).
    pub fn place_structure(&self, kind: StructureKind, start: (i32, i32), load: &dyn Fn(i32, i32) -> Arc<ChunkShape>) -> Result<Vec<([i32; 3], Block)>, PlaceFailure> {
        let forced = self.structures.forced(&self.density, kind, start)?;
        Ok(self.draw(forced, start, load))
    }

    /// `/place jigsaw <pool> <target> <max_depth> <pos>`: assembles the jigsaw pool `pool` (e.g.
    /// "minecraft:trail_ruins/tower", namespace optional) from `at`, its first piece joined at its
    /// jigsaw named `target` (e.g. "minecraft:bottom"), `max_depth` pieces deep; same output.
    /// Err(Unknown) for a pool that does not exist, Err(Failed) when the start element drawn has
    /// no jigsaw named `target`.
    pub fn place_jigsaw(&self, pool: &str, target: &str, max_depth: u32, at: [i32; 3], load: &dyn Fn(i32, i32) -> Arc<ChunkShape>) -> Result<Vec<([i32; 3], Block)>, PlaceFailure> {
        let forced = self.structures.forced_jigsaw(&self.density, pool, target, max_depth, at)?;
        Ok(self.draw(forced, (at[0] >> 4, at[2] >> 4), load))
    }

    fn draw(&self, forced: Forced, start: (i32, i32), load: Load) -> Vec<([i32; 3], Block)> {
        let chunks = forced.bb.map_or((start.0, start.1, start.0, start.1), |b: BoundingBox| (b.x0 >> 4, b.z0 >> 4, b.x1 >> 4, b.z1 >> 4));
        let reach = ((chunks.2 - chunks.0 + 3) * (chunks.3 - chunks.1 + 3)) as usize;
        let limit = (reach * 4).max(LIMIT);
        let stage = OnDemand { generator: self, forced, chunks, load, loaded: SourceCache::new(limit), pieced: SourceCache::new(limit), staged: SourceCache::new(limit * 10) };
        stage.writes()
    }
}
