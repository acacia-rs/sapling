//! The structure stage of one chunk (`WorldGenerator::postProcessStructureFeatures` 14196b2c0):
//! per feature, in registry order, the chunk's generator reseeded with the large-feature chunk
//! seed, then every start whose box reaches the chunk.

use std::sync::Arc;

use super::canvas::{Canvas, Neighbours, Spawners};
use super::jigsaw::{ElementKind, Piece};
use super::mineshaft::{self, MESA_BIOMES, Mineshaft};
use super::monument::{self, Monument};
use super::probe::Probe;
use super::scattered::Scattered;
use super::stronghold::{self, Stronghold};
use super::{BoundingBox, StructureKind, Structures, template, treasure};
use crate::biome::ChunkBiomes;
use crate::block::Block;
use crate::density::Density;
use crate::feature::ElementWorld;
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

/// What a chunk's structure stage did besides its own blocks.
pub struct Placed {
    /// The heights the pieces may have touched, for `rejoin_connections`.
    pub touched: (i32, i32),
    /// Pieces' features' writes into the neighbours: (position, block, block replaced).
    pub spilled: Vec<([i32; 3], Block, Block)>,
    /// Mineshaft floors' writes into the neighbours, likewise.
    pub unclipped: Vec<([i32; 3], Block, Block)>,
    /// Spider corridors whose spawner is placed by the end of this chunk.
    pub spawners: Spawners,
}

impl Structures {
    /// The pieces reaching the chunk, reading past it through `neighbours` (and `features` for
    /// the features some pieces place).
    pub fn place(
        &self,
        density: &Density,
        shape: &mut ChunkShape,
        chunk: (i32, i32),
        neighbours: Neighbours,
        biomes: &ChunkBiomes,
        features: &ElementWorld,
    ) -> Option<Placed> {
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
        let mut mineshafts: Vec<Arc<Mineshaft>> = self
            .starts_near(density, StructureKind::Mineshaft, chunk, MINESHAFT_REACH)
            .into_iter()
            .map(|start| self.mineshaft(density, start))
            .filter(|m| m.bb.intersects(&column))
            .collect();
        // BDS visits a chunk's starts in its start map's order, which follows the load order; without
        // one, descending x then z matched the oracle best.
        match neighbours.shaft_order {
            Some(order) => mineshafts.sort_by_key(|m| order.iter().position(|start| *start == m.start)),
            None => mineshafts.reverse(),
        }
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
        let mut canvas = Canvas::new(shape, chunk, neighbours.carved).with_placed(neighbours.placed).with_features(features);
        let seeds = LargeFeatureSeed::new(self.level_seed);
        let seed = seeds.chunk(chunk.0, chunk.1);
        let chunk_random = |(x, z): (i32, i32)| Mt19937::new(seeds.chunk(x, z));
        let mut placed: Vec<BoundingBox> = mineshafts.iter().map(|m| m.bb).chain(strongholds.iter().map(|s| s.bb)).collect();
        // A replayed run's state is what the kinds post-processed chunk by chunk read past the chunk.
        (canvas.read_placed, canvas.limit_view) = (neighbours.live, neighbours.live);
        if !villages.is_empty() {
            Self::place_villages(density, &mut canvas, &villages, seed, &(neighbours.generated_tops)());
        }
        let mut random = chunk_random(chunk);
        canvas.limit_view = true;
        for stronghold in &strongholds {
            stronghold.post_process(&mut canvas, &mut random);
        }
        (canvas.read_placed, canvas.limit_view) = (false, false);
        let mut random = chunk_random(chunk);
        super::trace::label(|| "temple".into());
        for temple in &temples {
            placed.extend(temple.post_process(&mut canvas, &mut random, &chunk_random));
        }
        self.place_igloos(&mut canvas, &igloos);
        let mut random = chunk_random(chunk);
        // Their supports read neighbours BDS post-processed earlier with those neighbours' pieces;
        // other kinds matched worse that way (village trees, pyramid heights).
        (canvas.read_placed, canvas.limit_view) = (true, true);
        let mut spawners = if mineshafts.is_empty() { Spawners::new() } else { (neighbours.spawners)() };
        for mineshaft in &mineshafts {
            mineshaft.post_process(&mut canvas, &mut random, biomes, &mut spawners);
        }
        (canvas.read_placed, canvas.limit_view) = (neighbours.live, neighbours.live);
        Self::place_mansions(&mut canvas, &mansions);
        (canvas.read_placed, canvas.limit_view) = (false, false);
        super::trace::label(|| "treasure".into());
        if treasure {
            treasure::place(&mut canvas, &mut chunk_random(chunk), chunk);
        }
        Self::place_outposts(&mut canvas, &outposts, seed);
        let mut random = chunk_random(chunk);
        super::trace::label(|| "monument".into());
        canvas.limit_view = true;
        for monument in &monuments {
            monument.post_process(&mut canvas, &mut random);
            placed.push(monument.building.bb);
        }
        canvas.limit_view = false;
        Self::place_ocean_ruins(&mut canvas, &ruins, seed);
        self.place_shipwrecks(&mut canvas, &shipwrecks);
        super::trace::label(|| "ruined portal".into());
        self.place_portals(density, &mut canvas, &portals);
        (canvas.read_placed, canvas.limit_view) = (neighbours.live, neighbours.live);
        Self::place_ancient_cities(&mut canvas, &cities, seed);
        (canvas.read_placed, canvas.limit_view) = (false, false);
        self.place_v2(&mut canvas, &v2);
        let touched = placed.iter().fold(canvas.touched, |(lo, hi), b| (lo.min(b.y0 - 1), hi.max(b.y1 + 1)));
        Some(Placed { touched, spilled: std::mem::take(&mut canvas.spilled), unclipped: std::mem::take(&mut canvas.unclipped), spawners })
    }

    /// The neighbours BDS post-processes after `chunk` (decoration phases) that share a
    /// mineshaft piece with it: their floor writes land in it after its own pieces.
    pub fn later_mineshaft_writers(&self, density: &Density, chunk: (i32, i32)) -> Vec<(i32, i32)> {
        let column = BoundingBox::chunk(chunk.0, chunk.1);
        let starts = self.starts_near(density, StructureKind::Mineshaft, chunk, MINESHAFT_REACH + 1);
        let shafts: Vec<Arc<Mineshaft>> = starts.into_iter().map(|s| self.mineshaft(density, s)).filter(|m| m.bb.intersects(&column)).collect();
        let shared = |n: (i32, i32)| {
            let other = BoundingBox::chunk(n.0, n.1);
            shafts.iter().any(|m| m.pieces.iter().any(|p| p.frame.bb.intersects(&column) && p.frame.bb.intersects(&other)))
        };
        let later = |n: (i32, i32)| crate::feature::phase(n) > crate::feature::phase(chunk);
        (chunk.0 - 1..=chunk.0 + 1).flat_map(|x| (chunk.1 - 1..=chunk.1 + 1).map(move |z| (x, z))).filter(|&n| later(n) && shared(n)).collect()
    }

    /// The chunks around `chunk` (not itself) holding a feature element's piece, whose features
    /// may write into it.
    pub fn feature_sources(&self, density: &Density, chunk: (i32, i32)) -> Vec<(i32, i32)> {
        let mut sources = Vec::new();
        for (_, (x, _, z)) in self.feature_pieces(density, chunk) {
            let source = (x >> 4, z >> 4);
            if source != chunk && !sources.contains(&source) {
                sources.push(source);
            }
        }
        sources
    }

    /// The village pieces meeting `chunk` in post-process order, described, for probes.
    pub fn village_pieces(&self, density: &Density, chunk: (i32, i32)) -> Vec<String> {
        let column = BoundingBox::chunk(chunk.0, chunk.1);
        let villages = self.villages_near(density, chunk);
        let pieces = villages.iter().flat_map(|v| v.pieces.iter()).filter(|p| p.bb.intersects(&column));
        pieces.map(|p| format!("{} {:?} {:?} depth {} rigid {}", p.element.name, p.origin, p.bb, p.depth, p.is_rigid())).collect()
    }

    /// A mineshaft start's pieces in generation order, described, for probes.
    pub fn mineshaft_pieces(&self, density: &Density, start: (i32, i32)) -> Vec<(String, BoundingBox)> {
        self.mineshaft(density, start).pieces.iter().map(|p| (format!("{:?} depth {} {:?}", p.frame.orientation, p.frame.depth, p.kind), p.frame.bb)).collect()
    }

    /// The feature elements' pieces in the 3x3 chunks around `chunk`: feature, origin.
    pub fn feature_pieces(&self, density: &Density, chunk: (i32, i32)) -> Vec<(String, (i32, i32, i32))> {
        let area = BoundingBox::new(chunk.0 * 16 - 16, i32::MIN, chunk.1 * 16 - 16, chunk.0 * 16 + 31, i32::MAX, chunk.1 * 16 + 31);
        let near = |bb: &BoundingBox| bb.intersects(&area);
        let mut found = Vec::new();
        let mut add = |list: &[Piece]| {
            for p in list.iter().filter(|p| near(&p.bb)) {
                if let ElementKind::Feature(name) = &p.element.kind {
                    found.push((name.clone(), p.origin));
                }
            }
        };
        self.v2_reaching(density, chunk, &near).iter().for_each(|(_, p)| add(p));
        self.villages_reaching(density, chunk, &near).iter().for_each(|v| add(&v.pieces));
        self.ancient_cities_reaching(density, chunk, &near).iter().for_each(|c| add(&c.pieces));
        found
    }

    /// The temple starting in `start`, by the point biome at its centre.
    fn scattered(&self, density: &Density, start: (i32, i32)) -> Option<Scattered> {
        let probe = Probe { density };
        Scattered::new(start, probe.biome(start.0 * 16 + 8, probe.surface(start), start.1 * 16 + 8))
    }

    /// Whether a block lies in the bounding box of a mineshaft, a stronghold or (`with_v2`) a v2
    /// structure start (its pieces' union): what geodes ask the world generator's
    /// `findStructureFeatureTypeAt` before placing (feature/geode.rs) and multiface features ask
    /// of the two tunnel kinds (14196ac20). Other kinds are not modelled (no oracle case yet).
    pub fn in_structure(&self, density: &Density, (x, y, z): (i32, i32, i32), with_v2: bool) -> bool {
        let chunk = (x.div_euclid(16), z.div_euclid(16));
        let kind = StructureKind::Mineshaft;
        let mut mineshafts = self.starts_near(density, kind, chunk, kind.reach()).into_iter();
        let mut strongholds = self.starts_near(density, StructureKind::Stronghold, chunk, STRONGHOLD_REACH).into_iter();
        mineshafts.any(|start| self.mineshaft(density, start).bb.contains(x, y, z))
            || strongholds.any(|start| self.stronghold(start).bb.contains(x, y, z))
            || with_v2
                && self.v2_reaching(density, chunk, &|_| true).iter().any(|(_, pieces)| {
                    pieces.iter().map(|p| p.bb).reduce(|a, b| a.union(&b)).is_some_and(|bb| bb.contains(x, y, z))
                })
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
