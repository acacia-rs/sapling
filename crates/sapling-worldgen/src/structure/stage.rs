//! The structure stage of one chunk (`WorldGenerator::postProcessStructureFeatures` 14196b2c0):
//! per feature, in registry order, the chunk's generator reseeded with the large-feature chunk
//! seed, then every start whose box reaches the chunk.

use std::sync::Arc;

use super::canvas::{Canvas, Neighbours, Past, Settled, Spawners};
use super::jigsaw::{ElementKind, Piece};
use super::mineshaft::{self, MESA_BIOMES, Mineshaft};
use super::monument::{self, Monument};
use super::probe::Probe;
use super::reaching::{Reaching, STRONGHOLD_REACH};
use super::scattered::{Scattered, Temple};
use super::stronghold::{self, Stronghold};
use super::{BoundingBox, StructureKind, Structures, template, treasure};
use crate::biome::ChunkBiomes;
use crate::block::Block;
use crate::density::Density;
use crate::feature::ElementWorld;
use crate::random::{LargeFeatureSeed, Mt19937};
use crate::terrain::ChunkShape;

/// What a chunk's post-process did besides its own blocks.
pub struct Placed {
    /// The heights the pieces may have touched, for `rejoin_connections`.
    pub touched: (i32, i32),
    /// Its writes into the neighbours in order: `setBlock`s, then its pieces' features'.
    pub past: Vec<Past>,
    /// Spider corridors whose spawner is placed by the end of this chunk.
    pub spawners: Spawners,
    /// In order: a cell a template pass decided (true) or a piece wrote afterwards (false).
    pub settled: Vec<((i32, i32, i32), bool)>,
    /// The height of each desert pyramid placed, by start.
    pub temple_heights: Vec<((i32, i32), i32)>,
}

impl Structures {
    /// The post-process of `chunk` over `shape`, reading past it through `neighbours` (and
    /// `features` for the features some pieces place).
    pub fn place(&self, density: &Density, reaching: &Reaching, shape: &mut ChunkShape, chunk: (i32, i32), neighbours: Neighbours, biomes: &ChunkBiomes, features: &ElementWorld) -> Placed {
        let Reaching { strongholds, temples, mineshafts, treasure, monuments, igloos, shipwrecks, portals, ruins, outposts, cities, villages, v2, mansions } = reaching;
        let mut mineshafts: Vec<&Arc<Mineshaft>> = mineshafts.iter().collect();
        if mineshafts.len() > 1 {
            // `foreachIntersectingStructureStart` walks the start map's list.
            let mut order: Vec<(i32, i32)> = mineshafts.iter().map(|m| m.start).collect();
            (neighbours.shaft_order)(&mut order);
            mineshafts.sort_by_key(|m| order.iter().position(|start| *start == m.start));
        }
        let mut canvas = Canvas::new(shape, chunk, neighbours.placed).with_features(features);
        let seeds = LargeFeatureSeed::new(self.level_seed);
        let seed = seeds.chunk(chunk.0, chunk.1);
        let chunk_random = || Mt19937::new(seed);
        let mut placed: Vec<BoundingBox> = mineshafts.iter().map(|m| m.bb).chain(strongholds.iter().map(|s| s.bb)).collect();
        if !villages.is_empty() {
            Self::place_villages(density, &mut canvas, villages, seed, &(neighbours.generated_tops)());
        }
        let mut random = chunk_random();
        for stronghold in strongholds {
            stronghold.post_process(&mut canvas, &mut random);
            canvas.upgrade(&stronghold.bb);
        }
        let mut random = chunk_random();
        super::trace::label(|| "temple".into());
        let mut temple_heights = Vec::new();
        for temple in temples {
            let start_random = neighbours.pyramid_from_start.then(|| Mt19937::new(seeds.chunk(temple.start().0, temple.start().1)));
            let Some(bb) = temple.post_process(&mut canvas, &mut random, neighbours.temple_height, start_random) else { continue };
            placed.push(bb);
            // Logged per start (dbg_canon.json `upgrades`): huts and jungle temples, mineshafts and
            // strongholds upgrade; desert pyramids, igloos and every other kind do not.
            match temple.temple {
                Temple::DesertPyramid => temple_heights.push((temple.start(), bb.y0)),
                _ => canvas.upgrade(&temple.bb),
            }
        }
        canvas.whole = true;
        self.place_igloos(&mut canvas, igloos);
        canvas.whole = false;
        let mut random = chunk_random();
        let mut spawners = if mineshafts.is_empty() { Spawners::new() } else { (neighbours.spawners)() };
        for mineshaft in &mineshafts {
            mineshaft.post_process(&mut canvas, &mut random, biomes, &mut spawners);
            canvas.upgrade(&mineshaft.bb);
        }
        Self::place_mansions(&mut canvas, mansions);
        super::trace::label(|| "treasure".into());
        if *treasure {
            treasure::place(&mut canvas, &mut chunk_random(), chunk);
        }
        Self::place_outposts(&mut canvas, outposts, seed);
        let mut random = chunk_random();
        super::trace::label(|| "monument".into());
        for monument in monuments {
            monument.post_process(&mut canvas, &mut random);
            placed.push(monument.building.bb);
        }
        Self::place_ocean_ruins(&mut canvas, ruins, seed);
        canvas.whole = true;
        self.place_shipwrecks(&mut canvas, shipwrecks);
        super::trace::label(|| "ruined portal".into());
        self.place_portals(density, &mut canvas, portals);
        canvas.whole = false;
        Self::place_ancient_cities(&mut canvas, cities, seed);
        self.place_v2(&mut canvas, v2);
        let touched = placed.iter().fold(canvas.touched, |(lo, hi), b| (lo.min(b.y0 - 1), hi.max(b.y1 + 1)));
        let mut past = std::mem::take(&mut canvas.unclipped);
        past.append(&mut canvas.spilled);
        Placed { touched, past, spawners, settled: canvas.record.take_settled(), temple_heights }
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
    pub(super) fn scattered(&self, density: &Density, start: (i32, i32)) -> Option<Scattered> {
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
            || with_v2 && self.in_v2(density, (x, y, z))
    }

    /// Whether a block lies in a v2 structure's box (its pieces' union). `findStructureFeatureTypeAt`
    /// (141efd3b0) asks these first and then answers with that structure's kind alone, so a mineshaft
    /// inside a trial chamber's box is no tunnel to a multiface feature (far out, logged live).
    pub fn in_v2(&self, density: &Density, (x, y, z): (i32, i32, i32)) -> bool {
        let chunk = (x.div_euclid(16), z.div_euclid(16));
        self.v2_reaching(density, chunk, &|_| true)
            .iter()
            .any(|(_, pieces)| pieces.iter().map(|p| p.bb).reduce(|a, b| a.union(&b)).is_some_and(|bb| bb.contains(x, y, z)))
    }

    pub(super) fn mineshaft(&self, density: &Density, start: (i32, i32)) -> Arc<Mineshaft> {
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

    pub(super) fn monument(&self, start: (i32, i32)) -> Arc<Monument> {
        self.monuments.lock().unwrap().entry(start).or_insert_with(|| Arc::new(monument::generate(self.level_seed, start))).clone()
    }
}

/// Fence, pane and bar connections and stair corners as the chunk and its neighbours end up
/// (`neighbour` reads them after their pieces), for the cells no template pass decided (`settled`).
pub fn rejoin_connections(shape: &mut ChunkShape, chunk: (i32, i32), heights: (i32, i32), neighbour: &dyn Fn(i32, i32, i32) -> Block, settled: &Settled) {
    template::rejoin(&mut Canvas::new(shape, chunk, neighbour), heights, settled);
}
