//! Which stage decides a cell, and whether sapling got it right.

use sapling_worldgen::Block;

use super::BdsCell;

/// The stage that decides a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    Shape,
    Floor,
    Deepslate,
    Veins,
    Surface,
    Carvers,
    Lakes,
}

pub enum Verdict {
    Match(Stage),
    Error(Stage),
    Later(&'static str),
}

/// What the oracle loop knows about one of our cells besides its block.
#[derive(Clone, Copy, Default)]
pub struct Context {
    /// Our carvers emptied it.
    pub carved: bool,
    /// Our lava lake rewrote it.
    pub laked: bool,
    /// BDS emptied it next to a feature block (geode interiors, lush-cave pools).
    pub feature_emptied: bool,
    /// See `column_reshaped`.
    pub reshaped: bool,
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

fn stage_of(block: Block, y: i32, context: Context) -> Stage {
    if context.laked {
        return Stage::Lakes;
    }
    if context.carved {
        return Stage::Carvers;
    }
    match block {
        Block::Bedrock => Stage::Floor,
        Block::Deepslate => Stage::Deepslate,
        Block::Stone if y < 9 => Stage::Deepslate,
        Block::Stone | Block::Air | Block::Water | Block::Lava => Stage::Shape,
        _ if VEIN_BLOCKS.contains(&block) => Stage::Veins,
        _ => Stage::Surface,
    }
}

pub fn judge(ours: Block, bds: &BdsCell, y: i32, context: Context) -> Verdict {
    match judge_cell(ours, bds, y, context) {
        Verdict::Error(Stage::Surface) if context.reshaped => Verdict::Later("surface under a solid-shape difference (carver miss, feature, shape)"),
        verdict => verdict,
    }
}

/// For every cell of a column (top down, as `cells`), whether ours and BDS disagree on what is solid
/// within 8 blocks of it or on the column top: a carver or lake miss, a feature or a shape
/// difference then reworked the surface. Feature blocks and freeze-pass snow and ice count as
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

fn judge_cell(ours: Block, bds: &BdsCell, y: i32, context: Context) -> Verdict {
    if bds.is(ours) {
        return Verdict::Match(stage_of(ours, y, context));
    }
    if ours.is_solid() && (bds.is_empty() || bds.is_fluid()) {
        // The iceberg feature cuts its shape out of any packed ice and snow, the surface's too.
        if matches!(ours, Block::PackedIce | Block::Snow) {
            return Verdict::Later("iceberg features");
        }
        if context.feature_emptied {
            return Verdict::Later("features (BDS emptied it next to a feature block)");
        }
        return Verdict::Error(Stage::Carvers);
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
        return Verdict::Error(if context.laked || context.carved { stage_of(ours, y, context) } else { Stage::Shape });
    }
    if ours == Block::Bedrock || theirs == Block::Bedrock {
        return Verdict::Error(Stage::Floor);
    }
    if let Some(later) = later_feature(ours, theirs, bds) {
        return Verdict::Later(later);
    }
    if VEIN_BLOCKS.contains(&ours) || VEIN_BLOCKS.contains(&theirs) {
        return Verdict::Error(Stage::Veins);
    }
    let base = |b: Block| matches!(b, Block::Stone | Block::Deepslate);
    if base(ours) && base(theirs) {
        return Verdict::Error(Stage::Deepslate);
    }
    Verdict::Error(Stage::Surface)
}

/// Solid terrain against solid terrain that a feature sapling does not run explains.
fn later_feature(ours: Block, theirs: Block, bds: &BdsCell) -> Option<&'static str> {
    let base = |b: Block| matches!(b, Block::Stone | Block::Deepslate);
    if base(ours) && VEIN_BLOCKS.contains(&theirs) && !matches!(theirs, Block::RawCopperBlock | Block::RawIronBlock) {
        return Some("ore features (or a missed vein)");
    }
    if base(ours) && matches!(theirs, Block::Dirt | Block::Gravel | Block::Calcite) {
        return Some("ore features (or a missed vein)");
    }
    if matches!(ours, Block::GrassBlock | Block::Snow | Block::Podzol | Block::CoarseDirt | Block::Mycelium) && theirs == Block::Dirt {
        return Some("trees (dirt under trunks)");
    }
    if ours == Block::Mud && matches!(theirs, Block::Dirt | Block::GrassBlock) {
        return Some("mangrove features (dirt, grass over mud)");
    }
    if matches!(ours, Block::GrassBlock | Block::CoarseDirt | Block::Dirt) && theirs == Block::Podzol {
        return Some("giant spruces (podzol around)");
    }
    if matches!(ours, Block::Snow | Block::Dirt | Block::GrassBlock | Block::Stone) && theirs == Block::PackedIce {
        return Some("ice spikes");
    }
    let disked = match ours {
        Block::Dirt | Block::GrassBlock | Block::CoarseDirt => matches!(bds.block.as_str(), "sand" | "gravel" | "clay"),
        Block::Gravel => matches!(bds.block.as_str(), "sand" | "dirt" | "clay"),
        Block::Stone => bds.block == "sand",
        _ => false,
    };
    if disked {
        return Some("sand, gravel, clay disks and dirt blobs");
    }
    let soil = matches!(ours, Block::Dirt | Block::GrassBlock | Block::CoarseDirt | Block::Gravel | Block::Sand);
    let blob = matches!(theirs, Block::Granite | Block::Tuff | Block::Calcite | Block::CopperOre | Block::IronOre);
    if soil && blob || VEIN_BLOCKS.contains(&ours) && theirs == Block::Calcite {
        return Some("ore features and geodes over surface or vein blocks");
    }
    None
}
