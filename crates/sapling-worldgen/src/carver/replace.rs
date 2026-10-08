//! Which blocks each carver may empty. Caves and canyons test different lists; names resolved
//! from the `VanillaBlockTypeIds` initialiser 1425f41e0 (docs/research/worldgen-bds-carvers.md).

use crate::block::Block;

/// `CaveFeature::canReplaceBlock` 148281400 (1.18 config). BDS lists the glazed terracottas,
/// not the plain ones, and leaves out granite, mud, snow and ice.
pub fn cave_replaceable(block: Block) -> bool {
    use Block::*;
    matches!(
        block,
        Stone | Dirt | CoarseDirt | Podzol | GrassBlock | Sandstone | RedSandstone | Mycelium | SnowLayer | PackedIce
            | Deepslate | Calcite | Sand | RedSand | Gravel | Tuff | IronOre | DeepslateIronOre | RawIronBlock | CopperOre
            | DeepslateCopperOre | RawCopperBlock
    )
}

/// The cave carver's 14ccd9470: sand, red sand, suspicious sand.
pub fn is_sand(block: Block) -> bool {
    matches!(block, Block::Sand | Block::RedSand | Block::SuspiciousSand)
}

/// 14ccdb858: what both carvers regrow grass on.
pub fn is_dirt(block: Block) -> bool {
    matches!(block, Block::Dirt | Block::CoarseDirt)
}

/// The canyon's list 14ccd27e8 (tested by 148280800): Java's overworld carver replaceables.
pub fn canyon_replaceable(block: Block) -> bool {
    use Block::*;
    matches!(
        block,
        Stone | Granite | Diorite | Andesite | Tuff | Deepslate | Calcite | Dirt | CoarseDirt | Mud | Podzol | Mycelium
            | GrassBlock | DirtWithRoots | Terracotta | WhiteTerracotta | OrangeTerracotta | YellowTerracotta
            | LightGrayTerracotta | BlueTerracotta | BrownTerracotta | RedTerracotta | Sandstone | RedSandstone | Sand
            | RedSand | SuspiciousSand | IronOre | DeepslateIronOre | RawIronBlock | CopperOre | DeepslateCopperOre
            | RawCopperBlock | Snow | SnowLayer | PackedIce | Gravel | Sulfur | Cinnabar
    )
}
