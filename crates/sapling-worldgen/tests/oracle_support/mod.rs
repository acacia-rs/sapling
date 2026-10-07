//! Reading oracle worlds and sorting every cell where sapling and BDS disagree into the stage
//! that decides it, or into the later stages (carvers, lakes, features, liquid flow) sapling
//! does not run yet.
#![allow(dead_code)]

mod tally;

use std::path::PathBuf;

use acacia_world::Chunk;
use sapling_leveldb::LevelDbWorld;
use sapling_worldgen::Block;

#[allow(unused_imports)]
pub use tally::{BiomeTally, Tally, TopTally};

pub struct Region {
    pub name: &'static str,
    x0: i32,
    z0: i32,
    x1: i32,
    z1: i32,
}

impl Region {
    pub fn new(name: &'static str, x0: i32, z0: i32, x1: i32, z1: i32) -> Self {
        Self { name, x0, z0, x1, z1 }
    }

    pub fn chunks(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        (self.x0..=self.x1).flat_map(move |x| (self.z0..=self.z1).map(move |z| (x, z)))
    }
}

/// The world and its `-run2` twin, or None when the oracle worlds are absent. `WORLDGEN_ORACLE_DIR`
/// overrides the default `worlds/oracle/`.
pub fn open(world: &str) -> Option<(LevelDbWorld, LevelDbWorld)> {
    let root = std::env::var_os("WORLDGEN_ORACLE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../worlds/oracle"));
    let (a, b) = (root.join(world), root.join(format!("{world}-run2")));
    if !a.exists() || !b.exists() {
        return None;
    }
    Some((LevelDbWorld::open_in_memory(&a).unwrap(), LevelDbWorld::open_in_memory(&b).unwrap()))
}

/// A saved cell: block name without `minecraft:`, its states, and the name of a liquid in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BdsCell {
    pub block: String,
    pub states: String,
    pub liquid: Option<String>,
}

pub fn bds_cell(world: &LevelDbWorld, chunk: &Chunk, x: i32, y: i32, z: i32) -> BdsCell {
    let registry = world.registry();
    let state = |id| registry.get(id).map_or(("?".to_string(), String::new()), |s| (s.name.trim_start_matches("minecraft:").to_string(), s.properties.to_string()));
    let (block, states) = state(chunk.block(x, y, z));
    let liquid = chunk.liquid(x, y, z);
    let liquid = (liquid != registry.air_id()).then(|| state(liquid).0);
    BdsCell { block, states, liquid }
}

impl BdsCell {
    fn is(&self, block: Block) -> bool {
        self.block == block.name() && self.states == block.states() && self.liquid.is_none()
    }

    fn is_empty(&self) -> bool {
        matches!(self.block.as_str(), "air" | "cave_air" | "void_air")
    }

    fn is_fluid(&self) -> bool {
        matches!(self.block.as_str(), "water" | "flowing_water" | "lava" | "flowing_lava")
    }

    /// A block the terrain stages can place (whatever its states).
    fn terrain(&self) -> Option<Block> {
        Block::from_name(&self.block).filter(|b| b.is_solid())
    }
}

/// The stage that decides a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    Shape,
    Floor,
    Deepslate,
    Veins,
    Surface,
}

const VEIN_BLOCKS: &[Block] = &[
    Block::Granite,
    Block::Tuff,
    Block::CopperOre,
    Block::DeepslateCopperOre,
    Block::RawCopperBlock,
    Block::IronOre,
    Block::DeepslateIronOre,
    Block::RawIronBlock,
];

fn stage_of(block: Block, y: i32) -> Stage {
    match block {
        Block::Bedrock => Stage::Floor,
        Block::Deepslate => Stage::Deepslate,
        Block::Stone if y < 9 => Stage::Deepslate,
        Block::Stone | Block::Air | Block::Water | Block::Lava => Stage::Shape,
        _ if VEIN_BLOCKS.contains(&block) => Stage::Veins,
        _ => Stage::Surface,
    }
}

pub enum Verdict {
    Match(Stage),
    Error(Stage),
    Later(&'static str),
}

/// Who decides a cell and whether sapling got it right. `reshaped`: BDS's column was solid in other
/// places above the cell (column_reshaped), so the surface builder saw a different column there.
pub fn judge(ours: Block, bds: &BdsCell, y: i32, reshaped: bool) -> Verdict {
    match judge_cell(ours, bds, y) {
        Verdict::Error(Stage::Surface) if reshaped => Verdict::Later("carvers, lakes, shape (column differs above)"),
        verdict => verdict,
    }
}

/// For every cell of a column (top down, as `cells`), whether ours and BDS disagree on what is solid
/// within 8 blocks of it or on the column top: carvers, lakes (whose borders turn to stone) or a
/// shape difference then reworked the surface. Feature blocks and freeze-pass snow and ice count as
/// either.
pub fn column_reshaped(ours_solid: impl Fn(i32) -> bool, cells: &[(i32, BdsCell)]) -> Vec<bool> {
    let bds_solid = |cell: &BdsCell| -> Option<bool> {
        if cell.is_empty() || cell.is_fluid() {
            return Some(false);
        }
        cell.terrain().filter(|b| !matches!(b, Block::SnowLayer | Block::Ice)).map(|_| true)
    };
    let differs: Vec<bool> = cells.iter().map(|(y, cell)| bds_solid(cell).is_some_and(|solid| solid != ours_solid(*y))).collect();
    let our_top = cells.iter().find(|(y, _)| ours_solid(*y)).map(|(y, _)| *y);
    let bds_top = cells.iter().find(|(_, cell)| bds_solid(cell) == Some(true)).map(|(y, _)| *y);
    (0..cells.len()).map(|i| our_top != bds_top || differs[i.saturating_sub(8)..(i + 9).min(cells.len())].iter().any(|d| *d)).collect()
}

fn judge_cell(ours: Block, bds: &BdsCell, y: i32) -> Verdict {
    if bds.is(ours) {
        return Verdict::Match(stage_of(ours, y));
    }
    if ours.is_solid() && (bds.is_empty() || bds.is_fluid()) {
        return Verdict::Later("carvers, lakes (BDS empty where ours is solid)");
    }
    if !ours.is_solid() && (bds.is_fluid() || bds.is_empty()) {
        return Verdict::Later("liquid flow (fluid cells differ)");
    }
    let Some(theirs) = bds.terrain() else {
        return Verdict::Later("features, structures (non-terrain block)");
    };
    if matches!(ours, Block::Water | Block::Air) && matches!(theirs, Block::Ice | Block::SnowLayer) {
        return Verdict::Later("freeze pass (ice, snow layers)");
    }
    if matches!(ours, Block::Water | Block::Air | Block::PackedIce | Block::Snow) && matches!(theirs, Block::PackedIce | Block::Snow) {
        return Verdict::Later("iceberg features");
    }
    if bds.liquid.is_some() {
        return Verdict::Later("features, structures (waterlogged)");
    }
    if !ours.is_solid() {
        return Verdict::Error(Stage::Shape);
    }
    if ours == Block::Bedrock || theirs == Block::Bedrock {
        return Verdict::Error(Stage::Floor);
    }
    let base = |b: Block| matches!(b, Block::Stone | Block::Deepslate);
    if base(ours) && VEIN_BLOCKS.contains(&theirs) && !matches!(theirs, Block::RawCopperBlock | Block::RawIronBlock) {
        return Verdict::Later("ore features (or a missed vein)");
    }
    if base(ours) && matches!(theirs, Block::Dirt | Block::Gravel | Block::Calcite) {
        return Verdict::Later("ore features (or a missed vein)");
    }
    if matches!(ours, Block::GrassBlock | Block::Snow | Block::Podzol | Block::CoarseDirt | Block::Mycelium) && theirs == Block::Dirt {
        return Verdict::Later("trees (dirt under trunks)");
    }
    if ours == Block::Mud && matches!(theirs, Block::Dirt | Block::GrassBlock) {
        return Verdict::Later("mangrove features (dirt, grass over mud)");
    }
    if matches!(ours, Block::GrassBlock | Block::CoarseDirt | Block::Dirt) && theirs == Block::Podzol {
        return Verdict::Later("giant spruces (podzol around)");
    }
    if matches!(ours, Block::Snow | Block::Dirt | Block::GrassBlock | Block::Stone) && theirs == Block::PackedIce {
        return Verdict::Later("ice spikes");
    }
    let disked = match ours {
        Block::Dirt | Block::GrassBlock | Block::CoarseDirt => matches!(bds.block.as_str(), "sand" | "gravel" | "clay"),
        Block::Gravel => matches!(bds.block.as_str(), "sand" | "dirt" | "clay"),
        Block::Stone => bds.block == "sand",
        _ => false,
    };
    if disked {
        return Verdict::Later("sand, gravel, clay disks and dirt blobs");
    }
    let soil = matches!(ours, Block::Dirt | Block::GrassBlock | Block::CoarseDirt | Block::Gravel | Block::Sand);
    let blob = matches!(theirs, Block::Granite | Block::Tuff | Block::Calcite | Block::CopperOre | Block::IronOre);
    if soil && blob || VEIN_BLOCKS.contains(&ours) && theirs == Block::Calcite {
        return Verdict::Later("ore features and geodes over surface or vein blocks");
    }
    if VEIN_BLOCKS.contains(&ours) || VEIN_BLOCKS.contains(&theirs) {
        return Verdict::Error(Stage::Veins);
    }
    if base(ours) && base(theirs) {
        return Verdict::Error(Stage::Deepslate);
    }
    Verdict::Error(Stage::Surface)
}
