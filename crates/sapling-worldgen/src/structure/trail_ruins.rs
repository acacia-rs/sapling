//! Trail ruins (the pack's `minecraft:trail_ruins` jigsaw: the tower pool, depth 7, within 80
//! blocks, starting 15 under the surface), assembled by the v2 placer; the bury adaptation belongs
//! to the terrain stage.

use std::sync::Arc;

use super::canvas::Canvas;
use super::jigsaw::{Piece, V2Start};
use super::probe::RawHeights;
use super::{BoundingBox, StructureKind, Structures};
use crate::density::Density;
use crate::random::JavaRandom;

const START: V2Start = V2Start { pool: "trail_ruins/tower", max_depth: 7, horizontal: 80, vertical: 80, start_y: -15, project: true };
/// 80 blocks plus a piece.
const REACH: i32 = 6;

pub struct TrailRuins {
    pub pieces: Vec<Piece>,
}

/// Where the biome test looks (the start piece's middle at its start height).
pub fn position(density: &Density, seed: i64, start: (i32, i32)) -> (i32, i32, i32) {
    let raw = RawHeights::new(density);
    START.start_piece(&mut JavaRandom::large_feature(seed, start.0, start.1), start, &|x, z| raw.at(x, z)).1
}

pub fn generate(density: &Density, seed: i64, start: (i32, i32)) -> TrailRuins {
    let raw = RawHeights::new(density);
    TrailRuins { pieces: START.assemble(&mut JavaRandom::large_feature(seed, start.0, start.1), start, &|x, z| raw.at(x, z)) }
}

impl Structures {
    /// Trail ruins with a piece meeting `chunk`.
    pub(super) fn trail_ruins_near(&self, density: &Density, chunk: (i32, i32)) -> Vec<Arc<TrailRuins>> {
        let column = BoundingBox::chunk(chunk.0, chunk.1);
        let starts = self.starts_near(density, StructureKind::TrailRuins, chunk, REACH);
        let mut cache = self.templates.trail_ruins.lock().unwrap();
        starts
            .into_iter()
            .map(|start| cache.entry(start).or_insert_with(|| Arc::new(generate(density, self.seed, start))).clone())
            .filter(|r| r.pieces.iter().any(|p| p.bb.intersects(&column)))
            .collect()
    }

    /// Every ruin piece meeting the canvas's chunk, in assembly order.
    pub(super) fn place_trail_ruins(&self, canvas: &mut Canvas, ruins: &[Arc<TrailRuins>]) {
        let column = canvas.bb;
        for piece in ruins.iter().flat_map(|r| r.pieces.iter()).filter(|p| p.bb.intersects(&column)) {
            piece.post_process_java(canvas, self.seed);
        }
    }
}
