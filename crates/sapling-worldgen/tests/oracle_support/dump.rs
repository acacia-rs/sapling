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
    terrain: Option<BufWriter<File>>,
    /// The current chunk and its BDS structure blocks, found on its first dumped cell.
    structures: Option<((i32, i32), Vec<(i32, i32, i32)>)>,
}

impl Dump {
    pub fn open(world: &str, region: &str) -> Self {
        let Some(dir) = std::env::var_os("SAPLING_ORACLE_DUMP") else { return Self { out: None, terrain: None, structures: None } };
        let name: String = region.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
        let file = File::create(std::path::Path::new(&dir).join(format!("{world}.{name}.tsv"))).expect("dump file");
        let mut out = BufWriter::new(file);
        writeln!(out, "x\ty\tz\tstable\tbefore\tours\tbds\ttwin\tfamily\tstructure\tstage\tsdist").unwrap();
        let terrain = File::create(std::path::Path::new(&dir).join(format!("{world}.{name}.terrain"))).expect("terrain dump file");
        Self { out: Some(out), terrain: Some(BufWriter::new(terrain)), structures: None }
    }

    /// A stable error of a stage before the features (scripts/terrain_residue.py): stage, blocks, and
    /// which of our stages touched the cell (c carved, s structured, l laked, f featured, r reshaped).
    fn terrain_cell(&mut self, (x, y, z): (i32, i32, i32), stage: Stage, ours: Block, bds: &BdsCell, context: Context) {
        let flags: String = [(context.carved, 'c'), (context.structured, 's'), (context.laked, 'l'), (context.featured, 'f'), (context.reshaped, 'r')]
            .iter()
            .filter_map(|&(on, flag)| on.then_some(flag))
            .collect();
        let liquid = bds.liquid.as_deref().map_or(String::new(), |l| format!("+{l}"));
        let out = self.terrain.as_mut().expect("opened with the dump");
        writeln!(out, "{x}\t{y}\t{z}\t{stage:?}\t{}\t{}{liquid}\t{flags}", ours.name(), bds.block).unwrap();
    }

    /// Writes a cell our features or BDS's disagree on: stable cells judged feature or liquid errors
    /// (stage F or L) or fluid against solid (W), and unstable ones where either side's block is a feature family's. `structure`
    /// is `near` when a BDS structure block is close or our structure stage wrote the cell, else the
    /// unported kinds (`*`) whose area holds it; `sdist` is the Chebyshev distance to the nearest
    /// BDS structure block of the cell's chunk.
    #[allow(clippy::too_many_arguments)]
    pub fn cell(&mut self, at: (i32, i32, i32), before: Block, ours: Block, (bds, twin): (&BdsCell, &BdsCell), context: Context, areas: &[(StructureKind, (i32, i32), BoundingBox)], ported: &[StructureKind], (world, chunk): (&LevelDbWorld, &Chunk)) {
        if self.out.is_none() || bds.is(ours) {
            return;
        }
        let stable = bds == twin;
        let verdict = judge(ours, bds, at.1, context);
        if let Verdict::Error(stage) = verdict
            && stable
            && !matches!(stage, Stage::Features | Stage::Liquids)
        {
            self.terrain_cell(at, stage, ours, bds, context);
        }
        let liquid = stable && matches!(verdict, Verdict::Error(Stage::Liquids));
        // Fluid against solid (a spring's rock cell, a flow over a block that differs): judged as shape or carvers.
        let walled = stable && matches!(verdict, Verdict::Error(Stage::Carvers | Stage::Shape)) && (bds.is_fluid() || ours.is_fluid());
        let error = liquid || walled || matches!(verdict, Verdict::Error(Stage::Features) | Verdict::Missing(_));
        let fam = [bds.block.as_str(), ours.name(), twin.block.as_str()].into_iter().find_map(family);
        if !error && (stable || fam.is_none() && !context.featured) {
            return;
        }
        let (x, y, z) = at;
        let blocks = self.structure_blocks(world, chunk, (x.div_euclid(16), z.div_euclid(16)));
        let sdist = blocks.iter().map(|&(sx, sy, sz)| (sx - x).abs().max((sy - y).abs()).max((sz - z).abs())).min();
        let near = context.structured || sdist.is_some_and(|d| d <= NEAR);
        let structure = if near {
            "near".to_string()
        } else {
            let unported = areas.iter().filter(|(kind, _, area)| !ported.contains(kind) && area.contains(x, y, z));
            unported.map(|(kind, _, _)| format!("{}*", kind.name())).collect::<Vec<_>>().join(",")
        };
        let cell = |c: &BdsCell| format!("{}[{}]{}", c.block, c.states, c.liquid.as_deref().map_or(String::new(), |l| format!("+{l}")));
        let block = |b: Block| format!("{}[{}]{}", b.name(), b.states(), b.second_layer().map_or(String::new(), |l| format!("+{}", l.name())));
        let stable = if stable { "S" } else { "U" };
        let (before, ours, bds, twin) = (block(before), block(ours), cell(bds), cell(twin));
        let out = self.out.as_mut().unwrap();
        let stage = if liquid { "L" } else if walled { "W" } else { "F" };
        let sdist = sdist.map_or("-".to_string(), |d| d.to_string());
        writeln!(out, "{x}\t{y}\t{z}\t{stable}\t{before}\t{ours}\t{bds}\t{twin}\t{}\t{structure}\t{stage}\t{sdist}", fam.unwrap_or("-")).unwrap();
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
