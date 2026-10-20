//! The Nether biomes' `minecraft:surface_builder` and `surface_material_adjustments` (BDS 1.26.52 vanilla behaviour
//! packs), with the defaults BDS's loaders fill in (149d4f0a0): noise scale 1, and a missing `height_range` as the
//! whole column. Hell's range is `variable.sea_level - 3 ..= variable.sea_level + 2` at sea level 32, rounded.

use super::biome::{BASALT_DELTAS, CRIMSON_FOREST, HELL, SOULSAND_VALLEY, WARPED_FOREST};
use crate::block::Block;
use crate::surface::spec::{Adjustment, Materials};

/// An adjustment kept for a column applies at the y of `heights`, inclusive.
pub struct HeightAdjustment {
    pub adjustment: Adjustment,
    pub heights: [i32; 2],
}

/// `minecraft:capped` (CappedSurfaceBuilder, capped.rs).
pub struct Capped {
    pub floor: &'static [Block],
    pub ceiling: &'static [Block],
    pub beach: Option<Block>,
    pub sea: Block,
    pub foundation: Block,
}

pub enum Builder {
    /// `minecraft:overworld` (surface/overworld.rs).
    Default { sea_floor_depth: i32, materials: Materials, adjustments: &'static [HeightAdjustment] },
    Capped(Capped),
}

const WHOLE: [i32; 2] = [0, 127];

const fn materials(top: Block, sea_floor: Block) -> Materials {
    Materials { top, mid: Block::Netherrack, sea_floor, foundation: Block::Netherrack, sea: Block::Lava }
}

const fn tops(noise_scale: f32, range: [f32; 2], block: Block, mid: Option<Block>, heights: [i32; 2]) -> HeightAdjustment {
    let adjustment = Adjustment { noise_scale, range, top_material: Some(block), mid_material: mid, sea_floor_material: Some(block), foundation_material: None };
    HeightAdjustment { adjustment, heights }
}

const HELL_ADJUSTMENTS: &[HeightAdjustment] =
    &[tops(0.03125, [-1.0, -0.4], Block::Gravel, None, [29, 34]), tops(0.03125, [0.4, 1.0], Block::SoulSand, Some(Block::SoulSand), [29, 34])];
const CRIMSON_ADJUSTMENTS: &[HeightAdjustment] =
    &[tops(0.1, [0.4, 1.0], Block::Netherrack, None, WHOLE), tops(1.0, [-0.75, -0.5], Block::NetherWartBlock, None, WHOLE)];
const WARPED_ADJUSTMENTS: &[HeightAdjustment] = &[tops(0.1, [0.4, 1.0], Block::Netherrack, None, WHOLE)];

pub fn builder(biome: u32) -> Option<Builder> {
    Some(match biome as u8 {
        HELL => Builder::Default { sea_floor_depth: 7, materials: materials(Block::Netherrack, Block::Netherrack), adjustments: HELL_ADJUSTMENTS },
        CRIMSON_FOREST => Builder::Default { sea_floor_depth: 1, materials: materials(Block::CrimsonNylium, Block::CrimsonNylium), adjustments: CRIMSON_ADJUSTMENTS },
        WARPED_FOREST => Builder::Default { sea_floor_depth: 1, materials: materials(Block::WarpedNylium, Block::WarpedNylium), adjustments: WARPED_ADJUSTMENTS },
        SOULSAND_VALLEY => Builder::Capped(Capped {
            floor: &[Block::SoulSand, Block::SoulSoil],
            ceiling: &[Block::SoulSand, Block::SoulSoil],
            beach: Some(Block::Gravel),
            sea: Block::Lava,
            foundation: Block::Netherrack,
        }),
        BASALT_DELTAS => Builder::Capped(Capped { floor: &[Block::Basalt, Block::Blackstone], ceiling: &[Block::Basalt], beach: None, sea: Block::Lava, foundation: Block::Netherrack }),
        _ => return None,
    })
}
