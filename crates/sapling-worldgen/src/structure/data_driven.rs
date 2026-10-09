//! The v2 jigsaw structures, assembled by the Java-style placer (`jigsaw::V2Start`): trail ruins
//! (the pack's `minecraft:trail_ruins`), trial chambers (registered in C++, 141f82140) and
//! abandoned camps (the pack's `abandoned_camp` set). Their biome tests read the start piece's
//! middle; their terrain adaptations belong to the terrain stage (`adjust`).

use std::sync::Arc;

use super::canvas::Canvas;
use super::jigsaw::{ElementKind, Piece, SetEntry, StartHeight, V2Start, data};
use super::probe::{Probe, RawHeights};
use super::template::{Kind, Rotation, v2_block};
use super::{BoundingBox, StructureKind, Structures};
use crate::block::Block;
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

const V2_KINDS: [StructureKind; 3] = [StructureKind::TrailRuins, StructureKind::TrialChambers, StructureKind::AbandonedCamp];

/// How many chunks a structure's pieces reach from its start (its horizontal reach plus a piece).
fn reach(kind: StructureKind) -> i32 {
    match kind {
        StructureKind::TrailRuins => 6,
        StructureKind::TrialChambers => 9,
        _ => 4,
    }
}

/// A camp: surface projected from 0, Java's (and the pack's) vertical reach equal to the horizontal.
fn camp_start(entry: &'static SetEntry) -> V2Start {
    let reach = entry.horizontal;
    V2Start {
        pool: &entry.pool,
        max_depth: entry.max_depth,
        horizontal: reach,
        vertical: reach,
        height: StartHeight::Constant(0),
        project: true,
        padding: entry.padding,
        aliases: None,
    }
}

impl Structures {
    /// `kind`'s start in `chunk` and whether its biome test passes; camps pick their variant the
    /// way Java's `ChunkGenerator.tryGenerateStructure` picks from a set: by weight, with the
    /// large-feature `java.util.Random`, dropping each variant whose biome test fails.
    fn v2_start(&self, density: &Density, kind: StructureKind, chunk: (i32, i32)) -> Option<V2Start> {
        let probe = Probe { density };
        let raw = RawHeights::new(density);
        let biome_at = |start: &V2Start| {
            let (x, y, z) = start.start_piece(&mut JavaRandom::large_feature(self.seed, chunk.0, chunk.1), chunk, &|x, z| raw.at(x, z)).1;
            probe.biome(x, y, z)
        };
        match kind {
            StructureKind::TrailRuins => super::kind::TRAIL_RUINS_BIOMES.contains(&biome_at(&TRAIL_RUINS)).then_some(TRAIL_RUINS),
            StructureKind::TrialChambers => (!super::kind::DEEP_DARK.contains(&biome_at(&TRIAL_CHAMBERS))).then_some(TRIAL_CHAMBERS),
            _ => {
                let mut entries: Vec<&'static SetEntry> = data().sets["abandoned_camp"].iter().collect();
                let mut total: i32 = entries.iter().map(|e| e.weight).sum();
                let mut random = JavaRandom::large_feature(self.seed, chunk.0, chunk.1);
                while !entries.is_empty() {
                    let mut left = random.next_int(total);
                    let k = entries.iter().position(|e| {
                        left -= e.weight;
                        left < 0
                    });
                    let k = k.unwrap_or(entries.len() - 1);
                    let start = camp_start(entries[k]);
                    if entries[k].biomes.contains(&biome_at(&start)) {
                        return Some(start);
                    }
                    total -= entries[k].weight;
                    entries.remove(k);
                }
                None
            }
        }
    }

    /// Whether a v2 structure starts in `chunk`, its grid candidate already checked.
    pub(super) fn v2_starts(&self, density: &Density, kind: StructureKind, chunk: (i32, i32)) -> bool {
        self.v2_start(density, kind, chunk).is_some()
    }

    /// The pieces of every v2 structure with a piece meeting `chunk`.
    pub(super) fn v2_near(&self, density: &Density, chunk: (i32, i32)) -> Vec<Arc<Vec<Piece>>> {
        let column = BoundingBox::chunk(chunk.0, chunk.1);
        self.v2_reaching(density, chunk, &|bb| bb.intersects(&column)).into_iter().map(|(_, pieces)| pieces).collect()
    }

    /// The kind and pieces of every v2 structure around `chunk` with a piece that `near` accepts.
    pub(super) fn v2_reaching(&self, density: &Density, chunk: (i32, i32), near: &dyn Fn(&BoundingBox) -> bool) -> Vec<(StructureKind, Arc<Vec<Piece>>)> {
        let mut found = Vec::new();
        for kind in V2_KINDS {
            for at in self.starts_near(density, kind, chunk, reach(kind) + 1) {
                let pieces = self.templates.v2.lock().unwrap().get(&(kind, at)).cloned();
                let pieces = pieces.unwrap_or_else(|| {
                    let raw = RawHeights::new(density);
                    let start = self.v2_start(density, kind, at).expect("a v2 start");
                    let built = Arc::new(start.assemble(&mut JavaRandom::large_feature(self.seed, at.0, at.1), self.seed, at, &|x, z| raw.at(x, z)));
                    self.templates.v2.lock().unwrap().insert((kind, at), built.clone());
                    built
                });
                if pieces.iter().any(|p| near(&p.bb)) {
                    found.push((kind, pieces));
                }
            }
        }
        found
    }

    /// For probes: the template blocks of v2 pieces in `chunk` as (position, template name, unturned block, rotation, turned block).
    pub fn v2_template_cells(&self, density: &Density, chunk: (i32, i32)) -> Vec<([i32; 3], String, Block, u8, Block)> {
        let column = BoundingBox::chunk(chunk.0, chunk.1);
        let mut cells = Vec::new();
        for piece in self.v2_near(density, chunk).iter().flat_map(|s| s.iter()).filter(|p| p.bb.intersects(&column)) {
            let ElementKind::Template { template, .. } = &piece.element.kind else { continue };
            for b in template.blocks.iter().filter(|b| b.state.kind() == Kind::Block) {
                let (dx, dy, dz) = piece.rotation.apply(b.pos);
                let at = [piece.origin.0 + dx, piece.origin.1 + dy, piece.origin.2 + dz];
                if column.contains(at[0], at[1], at[2]) {
                    cells.push((at, template.name.clone(), b.state.block(Rotation::None), piece.rotation as u8, v2_block(b.state, piece.rotation)));
                }
            }
        }
        cells
    }

    /// Every piece meeting the canvas's chunk, in assembly order.
    pub(super) fn place_v2(&self, canvas: &mut Canvas, structures: &[Arc<Vec<Piece>>]) {
        let column = canvas.bb;
        for piece in structures.iter().flat_map(|s| s.iter()).filter(|p| p.bb.intersects(&column)) {
            piece.post_process_java(canvas, self.seed);
        }
    }
}
