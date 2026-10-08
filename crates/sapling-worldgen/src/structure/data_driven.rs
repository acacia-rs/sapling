//! The v2 jigsaw structures, assembled by the Java-style placer (`jigsaw::V2Start`): trail ruins
//! (the pack's `minecraft:trail_ruins`; the bury adaptation belongs to the terrain stage) and
//! trial chambers (registered in C++, 141f82140). Their biome tests read the start piece's middle.

use std::sync::Arc;

use super::canvas::Canvas;
use super::jigsaw::{Piece, StartHeight, V2Start};
use super::probe::RawHeights;
use super::{BoundingBox, StructureKind, Structures};
use crate::density::Density;
use crate::random::JavaRandom;

const TRAIL_RUINS: V2Start = V2Start {
    pool: "trail_ruins/tower",
    max_depth: 7,
    horizontal: 80,
    vertical: 80,
    height: StartHeight::Constant(-15),
    project: true,
    padding: 0,
    aliases: None,
};

const TRIAL_CHAMBERS: V2Start = V2Start {
    pool: "trial_chambers/chamber/end",
    max_depth: 20,
    horizontal: 116,
    vertical: 116,
    height: StartHeight::Uniform(-40, -20),
    project: false,
    padding: 10,
    aliases: Some("trial_chambers"),
};

/// The structure's start and how many chunks its pieces reach (its horizontal reach plus a piece).
fn start(kind: StructureKind) -> (&'static V2Start, i32) {
    match kind {
        StructureKind::TrailRuins => (&TRAIL_RUINS, 6),
        StructureKind::TrialChambers => (&TRIAL_CHAMBERS, 9),
        _ => unreachable!("{kind:?} is not a v2 jigsaw structure"),
    }
}

impl Structures {
    /// Where `kind`'s biome test looks for a start in `chunk`.
    pub(super) fn v2_position(&self, density: &Density, kind: StructureKind, chunk: (i32, i32)) -> (i32, i32, i32) {
        let raw = RawHeights::new(density);
        start(kind).0.start_piece(&mut JavaRandom::large_feature(self.seed, chunk.0, chunk.1), chunk, &|x, z| raw.at(x, z)).1
    }

    /// The pieces of every trail ruin and trial chamber with a piece meeting `chunk`.
    pub(super) fn v2_near(&self, density: &Density, chunk: (i32, i32)) -> Vec<Arc<Vec<Piece>>> {
        let column = BoundingBox::chunk(chunk.0, chunk.1);
        let mut found = Vec::new();
        for kind in [StructureKind::TrailRuins, StructureKind::TrialChambers] {
            let (v2, reach) = start(kind);
            for at in self.starts_near(density, kind, chunk, reach) {
                let pieces = self.templates.v2.lock().unwrap().get(&(kind, at)).cloned();
                let pieces = pieces.unwrap_or_else(|| {
                    let raw = RawHeights::new(density);
                    let built = Arc::new(v2.assemble(&mut JavaRandom::large_feature(self.seed, at.0, at.1), at, &|x, z| raw.at(x, z)));
                    self.templates.v2.lock().unwrap().insert((kind, at), built.clone());
                    built
                });
                if pieces.iter().any(|p| p.bb.intersects(&column)) {
                    found.push(pieces);
                }
            }
        }
        found
    }

    /// Every piece meeting the canvas's chunk, in assembly order.
    pub(super) fn place_v2(&self, canvas: &mut Canvas, structures: &[Arc<Vec<Piece>>]) {
        let column = canvas.bb;
        for piece in structures.iter().flat_map(|s| s.iter()).filter(|p| p.bb.intersects(&column)) {
            piece.post_process_java(canvas, self.seed);
        }
    }
}
