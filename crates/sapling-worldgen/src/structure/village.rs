//! Villages (`VillageFeature::createStructureStart` 1480a83a0 -> 141f79aa0): the type from the
//! biome at the chunk's middle (desert, snowy, savanna, taiga, else plains), a rotation, a 2%
//! abandoned roll, a town centre from the type's pool, then the legacy jigsaw placer to depth 6.
//! Pieces stand on the raw terrain heights (`WorldGenerator::computeChunkHeightMap` 14196d010).

use std::sync::Arc;

use super::canvas::Canvas;
use super::jigsaw::{Assembler, Piece, Variant, VillageClass, data, middle};
use super::probe::{Probe, RawHeights};
use super::template::Rotation;
use super::{BoundingBox, StructureKind, Structures};
use crate::block::Block;
use crate::density::Density;
use crate::random::Mt19937;

/// Six pieces deep reach about 120 blocks from the town centre.
const REACH: i32 = 8;

pub struct Village {
    pub pieces: Vec<Piece>,
}

/// The `minecraft:village_type` of a biome, as the start's switch numbers them.
fn village_type(biome: u32) -> &'static str {
    match biome {
        2 => "desert",
        12 => "snowy",
        35 => "savanna",
        5 | 19 | 30 | 31 => "taiga",
        _ => "plains",
    }
}

pub fn generate(density: &Density, random: &mut Mt19937, start: (i32, i32)) -> Village {
    let probe = Probe { density };
    let surface = probe.surface(start);
    let kind = village_type(probe.biome(start.0 * 16 + 8, surface, start.1 * 16 + 8));
    let rotation = Rotation::from_index(random.next_int(4));
    let abandoned = random.next_float() < 0.02;
    let pool = if abandoned { format!("village/abandoned/{kind}/town_centers") } else { format!("village/{kind}/town_centers") };
    let pool = data().pool(&pool).expect("village town centres");
    let element = pool.list[random.next_int(pool.list.len() as i32) as usize];
    let raw = RawHeights::new(density);
    let height = |x: i32, z: i32| raw.at(x, z);
    let mut piece = Piece::new(element, (start.0 * 16, surface, start.1 * 16), rotation, 0);
    let bb = piece.bb;
    piece.shift_y(height(middle(bb.x0, bb.x1), middle(bb.z0, bb.z1)) - 1 - bb.y0);
    let mut assembler = Assembler::new(Variant::Legacy, 6, random, &height);
    assembler.start_legacy(piece);
    Village { pieces: assembler.pieces }
}

impl Structures {
    /// Villages with a piece meeting `chunk`.
    pub(super) fn villages_near(&self, density: &Density, chunk: (i32, i32)) -> Vec<Arc<Village>> {
        let column = BoundingBox::chunk(chunk.0, chunk.1);
        self.villages_reaching(density, chunk, &|bb| bb.intersects(&column))
    }

    /// Villages around `chunk` with a piece that `near` accepts.
    pub(super) fn villages_reaching(&self, density: &Density, chunk: (i32, i32), near: &dyn Fn(&BoundingBox) -> bool) -> Vec<Arc<Village>> {
        let starts = self.starts_near(density, StructureKind::Village, chunk, REACH + 1);
        let grid = StructureKind::Village.grid().unwrap();
        let mut cache = self.templates.villages.lock().unwrap();
        starts
            .into_iter()
            .map(|start| cache.entry(start).or_insert_with(|| Arc::new(generate(density, &mut grid.candidate_random(self.level_seed, start).1, start))).clone())
            .filter(|v| v.pieces.iter().any(|p| near(&p.bb)))
            .collect()
    }

    /// Every village piece meeting the canvas's chunk, in assembly order; `seed` is the chunk's
    /// structure seed, `tops` its pre-worldgen heightmap: what terrain matching reads while the
    /// chunk generates (placeInWorld 141eeb8d0, `LevelChunk` +0xf80), so a path bridges a carved cave.
    pub(super) fn place_villages(density: &Density, canvas: &mut Canvas, villages: &[Arc<Village>], seed: u32, tops: &[i16; 256]) {
        let b = canvas.bb;
        let heights = |x: i32, z: i32| tops[((z & 15) << 4 | (x & 15)) as usize] as i32;
        let top_material = |x: i32, y: i32, z: i32| {
            let biome = density.climate.biome_at(x, y, z);
            crate::surface::surface_spec(biome).map_or(Block::GrassBlock, |s| s.materials.top)
        };
        let class = VillageClass { top_material: &top_material };
        let mut random = Mt19937::new(seed);
        for piece in villages.iter().flat_map(|v| v.pieces.iter()).filter(|p| p.bb.intersects(&b)) {
            piece.post_process(canvas, seed, &mut random, &class, &heights);
        }
    }
}
