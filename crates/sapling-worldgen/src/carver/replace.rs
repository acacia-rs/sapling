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

/// The canyon's list 14ccd27e8 (tested by 148280800): Java's overworld carver replaceables.
pub fn canyon_replaceable(block: Block) -> bool {
    use Block::*;
    matches!(
        block,
        Stone | Granite | Tuff | Deepslate | Calcite | Dirt | CoarseDirt | Mud | Podzol | Mycelium | GrassBlock
            | Terracotta | WhiteTerracotta | OrangeTerracotta | YellowTerracotta | BrownTerracotta | RedTerracotta
            | LightGrayTerracotta | Sandstone | RedSandstone | Sand | RedSand | IronOre | DeepslateIronOre | RawIronBlock
            | CopperOre | DeepslateCopperOre | RawCopperBlock | Snow | SnowLayer | PackedIce | Gravel
    )
}
