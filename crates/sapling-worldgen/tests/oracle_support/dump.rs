//! Per-cell dump of feature disagreements (`SAPLING_ORACLE_DUMP=<dir>`), for breakdowns by family
//! (scripts/feature_residue.py). One TSV per world and region.

use std::fs::File;
use std::io::{BufWriter, Write};

use acacia_world::Chunk;
use sapling_leveldb::LevelDbWorld;
use sapling_worldgen::structure::{BoundingBox, StructureKind};
use sapling_worldgen::{Block, MIN_Y};

use super::features::family;
use super::judge::{Context, Stage, Verdict, judge};
use super::structures::structure_block;
use super::{BdsCell, bds_cell};

/// How far (Chebyshev, inside the cell's chunk) a BDS structure block makes a cell "next to a structure".
const NEAR: i32 = 6;

pub struct Dump {
    out: Option<BufWriter<File>>,
    /// The current chunk and its BDS structure blocks, found on its first dumped cell.
    structures: Option<((i32, i32), Vec<(i32, i32, i32)>)>,
}

impl Dump {
    pub fn open(world: &str, region: &str) -> Self {
        let Some(dir) = std::env::var_os("SAPLING_ORACLE_DUMP") else { return Self { out: None, structures: None } };
        let name: String = region.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
        let file = File::create(std::path::Path::new(&dir).join(format!("{world}.{name}.tsv"))).expect("dump file");
        let mut out = BufWriter::new(file);
        writeln!(out, "x\ty\tz\tstable\tbefore\tours\tbds\ttwin\tfamily\tstructure").unwrap();
        Self { out: Some(out), structures: None }
    }

    /// Writes a cell our features or BDS's disagree on: stable cells judged feature errors, and
    /// unstable ones where either side's block is a feature family's. `structure` is `near` when a
    /// BDS structure block is close or our structure stage wrote the cell, else the unported kinds
    /// (`*`) whose area holds it.
    #[allow(clippy::too_many_arguments)]
    pub fn cell(&mut self, at: (i32, i32, i32), before: Block, ours: Block, (bds, twin): (&BdsCell, &BdsCell), context: Context, areas: &[(StructureKind, (i32, i32), BoundingBox)], ported: &[StructureKind], (world, chunk): (&LevelDbWorld, &Chunk)) {
        if self.out.is_none() || bds.is(ours) {
            return;
        }
        let stable = bds == twin;
        let error = matches!(judge(ours, bds, at.1, context), Verdict::Error(Stage::Features));
        let fam = [bds.block.as_str(), ours.name(), twin.block.as_str()].into_iter().find_map(family);
        if !error && (stable || fam.is_none() && !context.featured) {
            return;
        }
        let (x, y, z) = at;
        let near = context.structured || self.structure_blocks(world, chunk, (x.div_euclid(16), z.div_euclid(16))).iter().any(|&(sx, sy, sz)| (sx - x).abs().max((sy - y).abs()).max((sz - z).abs()) <= NEAR);
        let structure = if near {
            "near".to_string()
        } else {
            let unported = areas.iter().filter(|(kind, _, area)| !ported.contains(kind) && area.contains(x, y, z));
            unported.map(|(kind, _, _)| format!("{}*", kind.name())).collect::<Vec<_>>().join(",")
        };
        let cell = |c: &BdsCell| format!("{}[{}]{}", c.block, c.states, c.liquid.as_deref().map_or(String::new(), |l| format!("+{l}")));
        let block = |b: Block| format!("{}[{}]{}", b.name(), b.states(), if b.is_waterlogged() { "+water" } else { "" });
        let stable = if stable { "S" } else { "U" };
        let (before, ours, bds, twin) = (block(before), block(ours), cell(bds), cell(twin));
        let out = self.out.as_mut().unwrap();
        writeln!(out, "{x}\t{y}\t{z}\t{stable}\t{before}\t{ours}\t{bds}\t{twin}\t{}\t{structure}", fam.unwrap_or("-")).unwrap();
    }

    fn structure_blocks(&mut self, world: &LevelDbWorld, chunk: &Chunk, at: (i32, i32)) -> &[(i32, i32, i32)] {
        if self.structures.as_ref().is_none_or(|(c, _)| *c != at) {
            let mut found = Vec::new();
            for y in MIN_Y..320 {
                for i in 0..256 {
                    let (x, z) = (at.0 * 16 + (i & 15), at.1 * 16 + (i >> 4));
                    if structure_block(&bds_cell(world, chunk, x, y, z), false) {
                        found.push((x, y, z));
                    }
                }
            }
            self.structures = Some((at, found));
        }
        &self.structures.as_ref().unwrap().1
    }
}
