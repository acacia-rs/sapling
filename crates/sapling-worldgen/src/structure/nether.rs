//! The Nether's structures, in `createNetherGenerator`'s (14880eb20) registry order: fortress, the
//! Nether ruined portal, bastion. Fortress and bastion share one grid and one selector (1480b9cf0):
//! the region's candidate, every quart around its corner in the kind's biome list, then
//! `nextInt(6) < 2` a fortress, else a bastion.

use std::sync::Arc;

use super::bastion::{self, Bastion};
use super::canvas::{Canvas, Spawners};
use super::fortress::{self, Fortress};
use super::ruined_portal;
use super::placement::{Grid, Spread};
use super::stage::Placed;
use super::start_cache::StartCache;
use crate::block::Block;
use crate::nether::{BASALT_DELTAS, CRIMSON_FOREST, HELL, SOULSAND_VALLEY, WARPED_FOREST};
use crate::random::{LargeFeatureSeed, Mt19937};
use crate::terrain::ChunkShape;

pub const NETHER_GRID: Grid = Grid::new(30, 4, 30_084_232, Spread::Linear);
/// The ruined portal's Nether grid (the feature's +0x17c, +0x180), the overworld's salt.
const PORTAL_GRID: Grid = Grid::new(25, 10, 40_552_231, Spread::Linear);
const FORTRESS_BIOMES: [u8; 5] = [HELL, SOULSAND_VALLEY, CRIMSON_FOREST, WARPED_FOREST, BASALT_DELTAS];
const BASTION_BIOMES: [u8; 4] = [HELL, SOULSAND_VALLEY, CRIMSON_FOREST, WARPED_FOREST];
/// Children stay within 112 blocks of the start piece; a piece reaches 19 past its door.
const FORTRESS_REACH: i32 = 9;
const STARTS_LIMIT: usize = 256;
pub const FLOOR: i32 = 0;
pub const TOP: i32 = 127;

/// The pre-zoom biome of a quart column (`NetherGenerator::quart_biome`).
pub type QuartBiome<'a> = &'a dyn Fn(i32, i32) -> u8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedKind {
    Fortress,
    Bastion,
}

/// The starts one chunk's post-process places, by kind.
#[derive(Default)]
pub struct NetherReaching {
    fortresses: Vec<Arc<Fortress>>,
    /// Ruined portal starts within a chunk: each places whole through the 3x3 block source.
    portals: Vec<(i32, i32)>,
    bastions: Vec<Arc<Bastion>>,
}

impl NetherReaching {
    fn is_empty(&self) -> bool {
        self.fortresses.is_empty() && self.portals.is_empty() && self.bastions.is_empty()
    }
}

pub struct NetherStructures {
    level_seed: u32,
    fortresses: StartCache<(i32, i32), Fortress>,
    bastions: StartCache<(i32, i32), Bastion>,
}

impl NetherStructures {
    pub fn new(seed: i64) -> Self {
        Self { level_seed: seed as u32, fortresses: StartCache::new(STARTS_LIMIT), bastions: StartCache::new(STARTS_LIMIT) }
    }

    pub fn bastion(&self, start: (i32, i32), quart_biome: QuartBiome) -> Option<Arc<Bastion>> {
        let mut random = self.select(SharedKind::Bastion, start, quart_biome)?;
        Some(self.bastions.get(start, || Bastion::generate(&mut random, start)))
    }

    /// Nether ruined portal starts within a chunk of `chunk` (no biome test, 14835c660).
    fn portals_near(&self, chunk: (i32, i32)) -> Vec<(i32, i32)> {
        let near = (-1..=1).flat_map(|dx| (-1..=1).map(move |dz| (chunk.0 + dx, chunk.1 + dz)));
        near.filter(|&c| PORTAL_GRID.candidate(self.level_seed, c) == c).collect()
    }

    /// The shared selector: the region's MT after its grid and pick draws when `kind` starts in
    /// `chunk`; the start's pieces go on drawing from it.
    pub fn select(&self, kind: SharedKind, chunk: (i32, i32), quart_biome: QuartBiome) -> Option<Mt19937> {
        let (candidate, mut random) = NETHER_GRID.candidate_random(self.level_seed, chunk);
        let biomes: &[u8] = match kind {
            SharedKind::Fortress => &FORTRESS_BIOMES,
            SharedKind::Bastion => &BASTION_BIOMES,
        };
        let (qx, qz) = (chunk.0 * 4, chunk.1 * 4);
        let quarts = [(qx - 1, qz - 1), (qx - 1, qz), (qx, qz - 1), (qx, qz)];
        if candidate != chunk || !quarts.iter().all(|&(x, z)| biomes.contains(&quart_biome(x, z))) {
            return None;
        }
        let fortress = random.next_int(6) < 2;
        (fortress == (kind == SharedKind::Fortress)).then_some(random)
    }

    pub fn fortress(&self, start: (i32, i32), quart_biome: QuartBiome) -> Option<Arc<Fortress>> {
        let mut random = self.select(SharedKind::Fortress, start, quart_biome)?;
        Some(self.fortresses.get(start, || fortress::generate(&mut random, start)))
    }

    /// The starts whose pieces meet `chunk`; None without any.
    pub fn reaching(&self, chunk: (i32, i32), quart_biome: QuartBiome) -> Option<NetherReaching> {
        let column = super::BoundingBox::chunk(chunk.0, chunk.1);
        let spacing = NETHER_GRID.spacing;
        let regions = |c: i32| (c - FORTRESS_REACH).div_euclid(spacing)..=(c + FORTRESS_REACH).div_euclid(spacing);
        let mut reaching = NetherReaching { portals: self.portals_near(chunk), ..Default::default() };
        for rx in regions(chunk.0) {
            for rz in regions(chunk.1) {
                let (start, _) = NETHER_GRID.candidate_random(self.level_seed, (rx * spacing, rz * spacing));
                let within = |reach: i32| (start.0 - chunk.0).abs() <= reach && (start.1 - chunk.1).abs() <= reach;
                if within(FORTRESS_REACH)
                    && let Some(fortress) = self.fortress(start, quart_biome).filter(|f| f.bb.intersects(&column))
                {
                    reaching.fortresses.push(fortress);
                }
                if within(bastion::REACH)
                    && let Some(bastion) = self.bastion(start, quart_biome).filter(|b| b.reaches(&column))
                {
                    reaching.bastions.push(bastion);
                }
            }
        }
        (!reaching.is_empty()).then_some(reaching)
    }

    /// The post-process of `chunk` over `shape` (`neighbour` reads the eight chunks around): per
    /// kind in registry order, the chunk's structure generator reseeded, then its starts.
    pub fn place(&self, reaching: &NetherReaching, shape: &mut ChunkShape, chunk: (i32, i32), neighbour: &dyn Fn(i32, i32, i32) -> Block) -> Placed {
        let mut canvas = Canvas::new(shape, chunk, neighbour).with_heights(FLOOR, TOP);
        let seed = LargeFeatureSeed::new(self.level_seed).chunk(chunk.0, chunk.1);
        let mut touched = (i32::MAX, i32::MIN);
        let mut random = Mt19937::new(seed);
        for fortress in &reaching.fortresses {
            fortress.post_process(&mut canvas, &mut random);
            touched = (touched.0.min(fortress.bb.y0 - 1), touched.1.max(fortress.bb.y1 + 1));
        }
        let seeds = LargeFeatureSeed::new(self.level_seed);
        canvas.whole = true;
        for &start in &reaching.portals {
            ruined_portal::place_nether(&mut canvas, &mut Mt19937::new(seeds.chunk(start.0, start.1)), start);
        }
        canvas.whole = false;
        let bastions: Vec<&Bastion> = reaching.bastions.iter().map(|b| &**b).collect();
        bastion::place(&mut canvas, &bastions, seed);
        let touched = (touched.0.min(canvas.touched.0), touched.1.max(canvas.touched.1));
        let mut past = std::mem::take(&mut canvas.unclipped);
        past.append(&mut canvas.spilled);
        Placed { touched, past, spawners: Spawners::new(), settled: canvas.record.take_settled(), temple_heights: Vec::new() }
    }

    /// `/locate structure fortress` (`getNearestGeneratedFeature` 141efb480): rings of chunks, not
    /// regions, out to 100, each ring's boundary x-major; the first start found wins.
    pub fn locate(&self, kind: SharedKind, (x, z): (i32, i32), quart_biome: QuartBiome) -> Option<(i32, i32)> {
        let (cx, cz) = (x >> 4, z >> 4);
        let spacing = NETHER_GRID.spacing;
        let mut regions = std::collections::HashMap::new();
        for ring in 0..=100i32 {
            for dx in -ring..=ring {
                for dz in -ring..=ring {
                    if dx.abs() != ring && dz.abs() != ring {
                        continue;
                    }
                    let chunk = (cx + dx, cz + dz);
                    let region = (chunk.0.div_euclid(spacing), chunk.1.div_euclid(spacing));
                    let start = *regions.entry(region).or_insert_with(|| {
                        let candidate = NETHER_GRID.candidate(self.level_seed, chunk);
                        self.select(kind, candidate, quart_biome).map(|_| candidate)
                    });
                    if start == Some(chunk) {
                        return start;
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{NetherStructures, SharedKind};

    /// BDS 1.26.5x `/locate structure fortress` in the Nether of seed 20261003: from (x, z), the block answered.
    const ANSWERS: [(i32, i32, i32, i32); 85] = [
        (-7680, -7680, -8160, -8080), (-7680, -5760, -7680, -5728), (-7680, -3840, -8048, -4128), (-7680, -1920, -7376, -2048), (-7680, 0, -8048, 256),
        (-7680, 1920, -7904, 2256), (-7680, 3840, -8096, 3904), (-7680, 5760, -7600, 5488), (-7680, 7680, -7840, 7424), (-5760, -7680, -5504, -7376),
        (-5760, -5760, -6384, -6048), (-5760, -3840, -5936, -3440), (-5760, -1920, -6032, -1856), (-5760, 0, -5200, 240), (-5760, 1920, -5520, 2400),
        (-5760, 3840, -5696, 4016), (-5760, 5760, -5664, 5760), (-5760, 7680, -5488, 8416), (-3840, -7680, -3840, -7600), (-3840, -5760, -3552, -6064),
        (-3840, -3840, -3280, -3248), (-3840, -1920, -3600, -1904), (-3840, 0, -4240, 208), (-3840, 1920, -3504, 2496), (-3840, 3840, -3616, 3584),
        (-3840, 5760, -3968, 6112), (-3840, 7680, -3680, 8032), (-1920, -7680, -2224, -7360), (-1920, -5760, -2080, -5696), (-1920, -3840, -1920, -3808),
        (-1920, -1920, -2336, -1824), (-1920, 0, -2224, 272), (-1920, 1920, -1600, 1472), (-1920, 3840, -2000, 3840), (-1920, 5760, -2048, 5600),
        (-1920, 7680, -2032, 7536), (0, -7680, 368, -7200), (0, -5760, -704, -5136), (0, -3840, 80, -4000), (0, -1920, -432, -2336),
        (0, 0, 240, 32), (0, 1920, -144, 1440), (0, 3840, 0, 3840), (0, 5760, 112, 5472), (0, 7680, 160, 7408),
        (1920, -7680, 1792, -8032), (1920, -5760, 2032, -6032), (1920, -3840, 2176, -4592), (1920, -1920, 1632, -1888), (1920, 0, 1488, 336),
        (1920, 1920, 2320, 1824), (1920, 3840, 2224, 4080), (1920, 5760, 1984, 6144), (1920, 7680, 1824, 7120), (3840, -7680, 3632, -7824),
        (3840, -5760, 3712, -5488), (3840, -3840, 3600, -4032), (3840, -1920, 3536, -1584), (3840, 0, 3664, 32), (3840, 1920, 3760, 1520),
        (3840, 3840, 3488, 3744), (3840, 5760, 3504, 5472), (3840, 7680, 3840, 7920), (5760, -7680, 5616, -7568), (5760, -5760, 5488, -6192),
        (5760, -3840, 5680, -3536), (5760, -1920, 6080, -1856), (5760, 0, 6080, 16), (5760, 1920, 5312, 1520), (5760, 3840, 5472, 3680),
        (5760, 5760, 6480, 6432), (5760, 7680, 5808, 7792), (7680, -7680, 7568, -8032), (7680, -5760, 7264, -5552), (7680, -3840, 7392, -4256),
        (7680, -1920, 6880, -2768), (7680, 0, 7088, 288), (7680, 1920, 7104, 1632), (7680, 3840, 7456, 3696), (7680, 5760, 7072, 6064),
        (7680, 7680, 7280, 8080), (123457, -98765, 123648, -98656), (-300000, 250000, -300208, 250000), (1000000, 1000000, 999744, 999968), (-77, 13, 240, 32),
    ];

    /// Every Nether biome takes a fortress, so the biome test never matters here.
    #[test]
    fn locate_matches_bds() {
        let structures = NetherStructures::new(20261003);
        let hell = |_: i32, _: i32| crate::nether::HELL;
        for (x, z, bx, bz) in ANSWERS {
            let chunk = structures.locate(SharedKind::Fortress, (x, z), &hell);
            assert_eq!(chunk.map(|(cx, cz)| (cx * 16, cz * 16)), Some((bx, bz)), "from {x} {z}");
        }
    }
}
