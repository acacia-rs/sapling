//! The structure stage of one chunk (`WorldGenerator::postProcessStructureFeatures` 14196b2c0):
//! per feature, in registry order, the chunk's generator reseeded with the large-feature chunk
//! seed, then every start whose box reaches the chunk.

use std::sync::Arc;

use super::canvas::Canvas;
use super::mineshaft::{self, MESA_BIOMES, Materials, Mineshaft};
use super::{BoundingBox, StructureKind, Structures, treasure};
use crate::biome::ChunkBiomes;
use crate::block::Block;
use crate::density::Density;
use crate::random::{LargeFeatureSeed, Mt19937};
use crate::terrain::ChunkShape;

/// Pieces stay within 80 blocks of the start's room plus a corridor, about 7 chunks.
const MINESHAFT_REACH: i32 = 7;

impl Structures {
    /// The pieces reaching the chunk, reading past it through `neighbour`; returns the heights
    /// they may have touched, for `rejoin_fences`.
    pub fn place(
        &self,
        density: &Density,
        shape: &mut ChunkShape,
        chunk: (i32, i32),
        neighbour: &dyn Fn(i32, i32, i32) -> Block,
        biomes: &ChunkBiomes,
    ) -> Option<(i32, i32)> {
        let column = BoundingBox::chunk(chunk.0, chunk.1);
        let mineshafts: Vec<Arc<Mineshaft>> = self
            .starts_near(density, StructureKind::Mineshaft, chunk, MINESHAFT_REACH)
            .into_iter()
            .map(|start| self.mineshaft(density, start))
            .filter(|m| m.bb.intersects(&column))
            .collect();
        let treasure = self.is_start(density, StructureKind::BuriedTreasure, chunk);
        if mineshafts.is_empty() && !treasure {
            return None;
        }
        let mut canvas = Canvas::new(shape, chunk, neighbour);
        let seed = LargeFeatureSeed::new(self.level_seed).chunk(chunk.0, chunk.1);
        let mut random = Mt19937::new(seed);
        for mineshaft in &mineshafts {
            mineshaft.post_process(&mut canvas, &mut random, biomes);
        }
        if treasure {
            treasure::place(&mut canvas, &mut Mt19937::new(seed), chunk);
        }
        Some(mineshafts.iter().fold((i32::MAX, i32::MIN), |(lo, hi), m| (lo.min(m.bb.y0 - 1), hi.max(m.bb.y1 + 1))))
    }

    fn mineshaft(&self, density: &Density, start: (i32, i32)) -> Arc<Mineshaft> {
        if let Some(found) = self.mineshafts.lock().unwrap().get(&start) {
            return found.clone();
        }
        let mesa = MESA_BIOMES.contains(&density.climate.biome_at(start.0 * 16, 0, start.1 * 16));
        let built = Arc::new(mineshaft::generate(&self.mineshaft_seed, start, mesa, &|qx, qz| density.preliminary_surface(qx, qz)));
        self.mineshafts.lock().unwrap().insert(start, built.clone());
        built
    }
}

/// Fence connections as the chunk and its neighbours end up (`neighbour` reads them after
/// their pieces): BDS's fences follow the changes beside them, across chunk borders too.
pub fn rejoin_fences(shape: &mut ChunkShape, chunk: (i32, i32), (low, high): (i32, i32), neighbour: &dyn Fn(i32, i32, i32) -> Block) {
    let mut canvas = Canvas::new(shape, chunk, neighbour);
    let b = canvas.bb;
    let woods = [false, true].map(|dark| Materials { dark, room_chance: 0.0 });
    let mut fences = Vec::new();
    for y in low.max(b.y0)..=high.min(b.y1) {
        for z in b.z0..=b.z1 {
            for x in b.x0..=b.x1 {
                let block = canvas.get(x, y, z);
                if let Some(materials) = woods.iter().find(|m| m.is_fence(block)) {
                    fences.push(((x, y, z), *materials));
                }
            }
        }
    }
    for ((x, y, z), materials) in fences {
        let fence = mineshaft::fence_at(materials, &canvas, (x, y, z));
        canvas.set(x, y, z, fence);
    }
}
