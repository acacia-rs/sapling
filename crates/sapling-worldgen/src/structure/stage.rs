//! The structure stage of one chunk (`WorldGenerator::postProcessStructureFeatures` 14196b2c0):
//! per feature, in registry order, the chunk's generator reseeded with the large-feature chunk
//! seed, then every start whose box reaches the chunk.

use std::sync::Arc;

use super::canvas::Canvas;
use super::mineshaft::{self, MESA_BIOMES, Mineshaft};
use super::monument::{self, Monument};
use super::probe::Probe;
use super::scattered::Scattered;
use super::stronghold::{self, Stronghold};
use super::{BoundingBox, StructureKind, Structures, template, treasure};
use crate::biome::ChunkBiomes;
use crate::block::Block;
use crate::density::Density;
use crate::random::{LargeFeatureSeed, Mt19937};
use crate::terrain::ChunkShape;

/// Pieces stay within 80 blocks of the start's room plus a corridor, about 7 chunks.
const MINESHAFT_REACH: i32 = 7;
/// Stronghold doors stay within 112 blocks of the start; pieces and the portal room fallback reach further.
const STRONGHOLD_REACH: i32 = 9;
/// A desert pyramid reaches one chunk past its start in x and z.
const TEMPLE_REACH: i32 = 1;
/// A monument's building spans its start chunk and two chunks either way.
const MONUMENT_REACH: i32 = 2;

impl Structures {
    /// The pieces reaching the chunk, reading past it through `neighbour`; returns the heights
    /// they may have touched, for `rejoin_connections`.
    pub fn place(
        &self,
        density: &Density,
        shape: &mut ChunkShape,
        chunk: (i32, i32),
        neighbour: &dyn Fn(i32, i32, i32) -> Block,
        biomes: &ChunkBiomes,
    ) -> Option<(i32, i32)> {
        let column = BoundingBox::chunk(chunk.0, chunk.1);
        let strongholds: Vec<Arc<Stronghold>> = self
            .starts_near(density, StructureKind::Stronghold, chunk, STRONGHOLD_REACH)
            .into_iter()
            .map(|start| self.stronghold(start))
            .filter(|s| s.bb.intersects(&column))
            .collect();
        let temples: Vec<Scattered> = self
            .starts_near(density, StructureKind::Temple, chunk, TEMPLE_REACH)
            .into_iter()
            .filter_map(|start| self.scattered(density, start))
            .filter(|t| t.bb.intersects(&column))
            .collect();
        let mineshafts: Vec<Arc<Mineshaft>> = self
            .starts_near(density, StructureKind::Mineshaft, chunk, MINESHAFT_REACH)
            .into_iter()
            .map(|start| self.mineshaft(density, start))
            .filter(|m| m.bb.intersects(&column))
            .collect();
        let treasure = self.is_start(density, StructureKind::BuriedTreasure, chunk);
        let monuments: Vec<Arc<Monument>> = self
            .starts_near(density, StructureKind::Monument, chunk, MONUMENT_REACH)
            .into_iter()
            .map(|start| self.monument(start))
            .filter(|m| m.building.bb.intersects(&column))
            .collect();
        let igloos = self.igloos_near(density, chunk);
        let ruins = self.ocean_ruins_near(density, chunk);
        let shipwrecks = self.shipwrecks_near(density, chunk);
        let portals = self.portals_near(density, chunk);
        let outposts = self.outposts_near(density, chunk);
        let cities = self.ancient_cities_near(density, chunk);
        let villages = self.villages_near(density, chunk);
        let v2 = self.v2_near(density, chunk);
        let mansions = self.mansions_near(density, chunk);
        let hardcoded = !(strongholds.is_empty() && temples.is_empty() && mineshafts.is_empty() && !treasure && monuments.is_empty());
        let templated = !(igloos.is_empty()
            && v2.is_empty()
            && mansions.is_empty()
            && ruins.is_empty()
            && shipwrecks.is_empty()
            && portals.is_empty()
            && outposts.is_empty()
            && cities.is_empty()
            && villages.is_empty());
        if !hardcoded && !templated {
            return None;
        }
        let mut canvas = Canvas::new(shape, chunk, neighbour);
        let seeds = LargeFeatureSeed::new(self.level_seed);
        let seed = seeds.chunk(chunk.0, chunk.1);
        let chunk_random = |(x, z): (i32, i32)| Mt19937::new(seeds.chunk(x, z));
        let mut placed: Vec<BoundingBox> = mineshafts.iter().map(|m| m.bb).chain(strongholds.iter().map(|s| s.bb)).collect();
        Self::place_villages(density, &mut canvas, &villages, seed);
        let mut random = chunk_random(chunk);
        for stronghold in &strongholds {
            stronghold.post_process(&mut canvas, &mut random);
        }
        let mut random = chunk_random(chunk);
        for temple in &temples {
            placed.extend(temple.post_process(&mut canvas, &mut random, &chunk_random));
        }
        self.place_igloos(&mut canvas, &igloos);
        let mut random = chunk_random(chunk);
        // BDS visits a chunk's starts in hash-map order; descending x then z matches it on the oracle.
        for mineshaft in mineshafts.iter().rev() {
            mineshaft.post_process(&mut canvas, &mut random, biomes);
        }
        Self::place_mansions(&mut canvas, &mansions);
        if treasure {
            treasure::place(&mut canvas, &mut chunk_random(chunk), chunk);
        }
        Self::place_outposts(&mut canvas, &outposts, seed);
        let mut random = chunk_random(chunk);
        for monument in &monuments {
            monument.post_process(&mut canvas, &mut random);
            placed.push(monument.building.bb);
        }
        Self::place_ocean_ruins(&mut canvas, &ruins, seed);
        self.place_shipwrecks(&mut canvas, &shipwrecks);
        self.place_portals(density, &mut canvas, &portals);
        Self::place_ancient_cities(&mut canvas, &cities, seed);
        self.place_v2(&mut canvas, &v2);
        Some(placed.iter().fold(canvas.touched, |(lo, hi), b| (lo.min(b.y0 - 1), hi.max(b.y1 + 1))))
    }

    /// The temple starting in `start`, by the point biome at its centre.
    fn scattered(&self, density: &Density, start: (i32, i32)) -> Option<Scattered> {
        let probe = Probe { density };
        Scattered::new(start, probe.biome(start.0 * 16 + 8, probe.surface(start), start.1 * 16 + 8))
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

    pub(super) fn stronghold(&self, start: (i32, i32)) -> Arc<Stronghold> {
        let build = || Arc::new(stronghold::generate(&mut self.mineshaft_seed.blueprint(start), start));
        self.stronghold_pieces.lock().unwrap().entry(start).or_insert_with(build).clone()
    }

    fn monument(&self, start: (i32, i32)) -> Arc<Monument> {
        self.monuments.lock().unwrap().entry(start).or_insert_with(|| Arc::new(monument::generate(self.level_seed, start))).clone()
    }
}

/// Fence, pane and bar connections and stair corners as the chunk and its neighbours end up
/// (`neighbour` reads them after their pieces): BDS's follow the changes beside them, across
/// chunk borders too.
pub fn rejoin_connections(shape: &mut ChunkShape, chunk: (i32, i32), heights: (i32, i32), neighbour: &dyn Fn(i32, i32, i32) -> Block) {
    template::rejoin(&mut Canvas::new(shape, chunk, neighbour), heights);
}
