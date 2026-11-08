//! The v2 jigsaw structures, assembled by the Java-style placer (`jigsaw::V2Start`): trail ruins
//! (the pack's `minecraft:trail_ruins`), trial chambers (registered in C++, 141f82140) and
//! abandoned camps (the pack's `abandoned_camp` set). Their biome tests read the start piece's
//! middle; their terrain adaptations belong to the terrain stage (`adjust`).

use std::sync::Arc;

use super::canvas::Canvas;
use super::jigsaw::{ElementKind, Piece, SetEntry, StartHeight, V2Start, data};
use super::probe::{Probe, RawHeights};
use super::template::{Kind, Rotation, v2_block};
use super::{BoundingBox, StructureKind, Structures, spawn_areas};
use crate::block::Block;
use crate::density::Density;
use crate::random::{JavaRandom, Xoroshiro};

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

    /// The middle (x, z) of `kind`'s start piece in `chunk`, which `/locate` names; None when its biome test fails.
    pub(super) fn v2_start_middle(&self, density: &Density, kind: StructureKind, chunk: (i32, i32)) -> Option<(i32, i32)> {
        let start = self.v2_start(density, kind, chunk)?;
        let raw = RawHeights::new(density);
        let (_, (x, _, z)) = start.start_piece(&mut JavaRandom::large_feature(self.seed, chunk.0, chunk.1), chunk, &|x, z| raw.at(x, z));
        Some((x, z))
    }

    /// Whether a v2 structure starts in `chunk`, its grid candidate already checked.
    pub(super) fn v2_starts(&self, density: &Density, kind: StructureKind, chunk: (i32, i32)) -> bool {
        self.v2_start(density, kind, chunk).is_some()
    }

    /// The kind and pieces of every v2 structure whose whole box meets `chunk`.
    pub(super) fn v2_near(&self, density: &Density, chunk: (i32, i32)) -> Vec<(Option<StructureKind>, Arc<Vec<Piece>>)> {
        let column = BoundingBox::chunk(chunk.0, chunk.1);
        let found = self.v2_reaching(density, chunk, &|_| true).into_iter();
        found.filter(|(_, pieces)| whole(pieces).intersects(&column)).map(|(kind, pieces)| (Some(kind), pieces)).collect()
    }

    /// The kind and pieces of every v2 structure around `chunk` with a piece that `near` accepts.
    pub(super) fn v2_reaching(&self, density: &Density, chunk: (i32, i32), near: &dyn Fn(&BoundingBox) -> bool) -> Vec<(StructureKind, Arc<Vec<Piece>>)> {
        let mut found = Vec::new();
        for kind in V2_KINDS {
            for at in self.starts_near(density, kind, chunk, reach(kind) + 1) {
                let pieces = self.v2_assembled(density, kind, at, || self.v2_start(density, kind, at).expect("a v2 start"));
                if pieces.iter().any(|p| near(&p.bb)) {
                    found.push((kind, pieces));
                }
            }
        }
        found
    }

    /// The pieces of `kind`'s start in `at`, which `start` gives.
    fn v2_assembled(&self, density: &Density, kind: StructureKind, at: (i32, i32), start: impl FnOnce() -> V2Start) -> Arc<Vec<Piece>> {
        self.templates.v2.get((kind, at), || {
            let raw = RawHeights::new(density);
            start().assemble(&mut JavaRandom::large_feature(self.seed, at.0, at.1), self.seed, at, &|x, z| raw.at(x, z))
        })
    }

    /// The pieces of `kind` started in `at` without its biome test; camps still pick their variant
    /// by biome, so None when none fits. With `start_y` (`/place structure`'s `ignoreStartHeight`)
    /// the start piece stands at that height, unprojected, in place of the kind's own.
    pub(super) fn forced_v2(&self, density: &Density, kind: StructureKind, at: (i32, i32), start_y: Option<i32>) -> Option<Arc<Vec<Piece>>> {
        let start = match kind {
            StructureKind::TrailRuins => TRAIL_RUINS,
            StructureKind::TrialChambers => TRIAL_CHAMBERS,
            _ => self.v2_start(density, kind, at)?,
        };
        let Some(y) = start_y else { return Some(self.v2_assembled(density, kind, at, || start)) };
        Some(self.assembled_at(density, start, at, y))
    }

    /// `/place structure`'s `minecraft:trail_ruins`, `minecraft:trial_chambers` or camp variant
    /// (`minecraft:abandoned_camp_forest`, whatever the biome, bds-u3place.txt) started at the block
    /// `at` ([`V2Start::assemble_from`]), with the block seeding its loot; None for another name.
    pub(super) fn commanded_v2(&self, density: &Density, name: &str, at: [i32; 3], keep_height: bool) -> Option<(Arc<Vec<Piece>>, [i32; 3])> {
        let start = match name {
            "minecraft:trail_ruins" => TRAIL_RUINS,
            "minecraft:trial_chambers" => TRIAL_CHAMBERS,
            camp => camp_start(data().sets["abandoned_camp"].iter().find(|entry| entry.name == camp)?),
        };
        let raw = RawHeights::new(density);
        let (pieces, loot_at) = start.assemble_from(self.seed, at, keep_height, &|x, z| raw.at(x, z));
        Some((Arc::new(pieces), loot_at))
    }

    /// `start`'s pieces with its start piece at height `y`, unprojected.
    fn assembled_at(&self, density: &Density, start: V2Start, at: (i32, i32), y: i32) -> Arc<Vec<Piece>> {
        let raw = RawHeights::new(density);
        let start = V2Start { height: StartHeight::Constant(y), project: false, ..start };
        Arc::new(start.assemble(&mut JavaRandom::large_feature(self.seed, at.0, at.1), self.seed, at, &|x, z| raw.at(x, z)))
    }

    /// For probes: the template blocks of v2 pieces in `chunk` as (position, template name, unturned block, rotation, turned block).
    pub fn v2_template_cells(&self, density: &Density, chunk: (i32, i32)) -> Vec<([i32; 3], String, Block, u8, Block)> {
        let column = BoundingBox::chunk(chunk.0, chunk.1);
        let mut cells = Vec::new();
        for piece in self.v2_near(density, chunk).iter().flat_map(|(_, s)| s.iter()).filter(|p| p.bb.intersects(&column)) {
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

    /// For probes: every piece of the v2 structures reaching `chunk`, in assembly order, then the
    /// pieces of ancient cities that meet the chunk.
    pub fn v2_pieces(&self, density: &Density, chunk: (i32, i32)) -> Vec<String> {
        let pieces = self.v2_near(density, chunk);
        let line = |p: &Piece| format!("{} at {:?} {:?} {:?} depth {}", p.element.name, p.origin, p.rotation, p.bb, p.depth);
        let column = BoundingBox::chunk(chunk.0, chunk.1);
        let cities = self.ancient_cities_near(density, chunk);
        let city = cities.iter().flat_map(|c| c.pieces.iter()).filter(|p| p.bb.intersects(&column));
        pieces.iter().flat_map(|(_, s)| s.iter()).chain(city).map(line).collect()
    }

    /// Every piece meeting the canvas's chunk, in assembly order, all drawing loot seeds from the
    /// chunk's one decoration generator; a `/place` command's ([`super::Reaching::commanded`])
    /// each from Java's random of the level seed forked at its start block (the start height
    /// before projection), anew for every piece (the command's per-piece, per-chunk action
    /// 144cfc4d0; bds-adplace's camps and trail ruins).
    pub(super) fn place_v2(&self, canvas: &mut Canvas, structures: &[(Option<StructureKind>, Arc<Vec<Piece>>)], commanded: Option<[i32; 3]>) {
        let column = canvas.bb;
        let mut random = Xoroshiro::decoration(self.seed, column.x0 >> 4, column.z0 >> 4);
        for (kind, pieces) in structures {
            canvas.dry = *kind == Some(StructureKind::TrialChambers);
            for piece in pieces.iter().filter(|p| p.bb.intersects(&column)) {
                match commanded {
                    Some([x, y, z]) => {
                        let mut forked = JavaRandom::new(self.seed).fork_at(x, y, z);
                        piece.post_process_java(canvas, self.seed, &mut || forked.next_long() as i32);
                    }
                    None => piece.post_process_java(canvas, self.seed, &mut || random.next_int()),
                }
            }
        }
        canvas.dry = false;
    }
}

/// Files each structure's spawn areas; BDS lists the data-driven ones before the hardcoded.
pub(super) fn file_v2(canvas: &mut Canvas, structures: &[(Option<StructureKind>, Arc<Vec<Piece>>)]) {
    for (kind, pieces) in structures {
        let Some(kind) = kind else { continue };
        spawn_areas::file(canvas, (v2_structure(*kind, pieces), None), whole(pieces), pieces.iter().map(|p| p.bb));
    }
}

/// A v2 structure's box: its pieces' union.
fn whole(pieces: &[Piece]) -> BoundingBox {
    pieces.iter().map(|p| p.bb).reduce(|a, b| a.union(&b)).expect("a start piece")
}

/// The structure's id; a camp's is its variant's, known by its start piece's pool.
fn v2_structure(kind: StructureKind, pieces: &[Piece]) -> &'static str {
    match kind {
        StructureKind::TrailRuins => "minecraft:trail_ruins",
        StructureKind::TrialChambers => "minecraft:trial_chambers",
        _ => {
            let start = &pieces[0].element.name;
            let variant = data().sets["abandoned_camp"].iter().find(|e| start.strip_prefix(e.pool.as_str()).is_some_and(|rest| rest.starts_with('/')));
            variant.map_or("minecraft:abandoned_camp", |e| e.name.as_str())
        }
    }
}

#[cfg(test)]
mod tests {
    /// Seed 20261003's chunk (-28, -27) against a BDS world that saved chunk -29 (20261003-y3tc): the
    /// copper grates over its west corridor, set into an aquifer, hold no water to flood it.
    #[test]
    fn trial_chamber_blocks_set_into_water_keep_none() {
        let shape = crate::OverworldGenerator::new(20261003).chunk_blocks(-28, -27);
        assert_eq!(shape.block(2, -11, 7).name(), "flower_pot", "the dead bush pot at -446 -11 -425");
        let wet: Vec<_> = (0..16).flat_map(|z| [0, 1].map(|x| (x, z))).filter(|&(x, z)| shape.block(x, -11, z).name() == "water").collect();
        assert!(wet.is_empty(), "BDS's corridor is dry: {wet:?}");
    }
}
