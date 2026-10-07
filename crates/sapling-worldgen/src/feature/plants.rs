//! `minecraft:single_block_feature` (`SingleBlockFeature::place` 143badfc0) and the placement and
//! survival rules of the blocks it places (`BlockLegacy::mayPlace` / `canSurvive` through the
//! world-generation API, slots 0x78 and 0x88).

use super::place::Ctx;
use super::spec::SingleBlock;
use super::view::View;
use crate::block::Block;

impl SingleBlock {
    /// `may_not_attach_to` fails before anything is drawn; the block is drawn next, even from one entry.
    pub fn place(&self, ctx: &mut Ctx, [x, y, z]: [i32; 3]) -> Option<[i32; 3]> {
        if self.not_on.contains(&ctx.view.block(x, y - 1, z)) {
            return None;
        }
        let total: i32 = self.blocks.iter().map(|&(_, weight)| weight).sum();
        let mut left = if total == 0 { 0 } else { ctx.random.next_int(total) };
        let &(block, _) = self.blocks.iter().find(|&&(_, weight)| {
            left -= weight;
            left < 0
        })?;
        let view = &mut *ctx.view;
        if self.placement && !may_place(view, [x, y, z], block) {
            return None;
        }
        if self.survival && !can_survive(view, [x, y, z], block) {
            return None;
        }
        if !self.may_replace.is_empty() && !self.may_replace.contains(&view.block(x, y, z)) {
            return None;
        }
        view.set_block(x, y, z, block).then_some([x, y, z])
    }
}

/// Soil a bush-like plant stands on (Java's `BlockTags.DIRT` among the blocks sapling places).
fn is_soil(block: Block) -> bool {
    matches!(block, Block::GrassBlock | Block::Dirt | Block::CoarseDirt | Block::Podzol | Block::Mycelium | Block::Mud)
}

fn is_dry_soil(block: Block) -> bool {
    is_soil(block)
        || matches!(
            block,
            Block::Sand
                | Block::RedSand
                | Block::Terracotta
                | Block::WhiteTerracotta
                | Block::OrangeTerracotta
                | Block::YellowTerracotta
                | Block::BrownTerracotta
                | Block::RedTerracotta
                | Block::LightGrayTerracotta
        )
}

/// Whether `block` may stand at `at`; a double plant's lower half also needs air above.
pub fn may_place(view: &View, [x, y, z]: [i32; 3], block: Block) -> bool {
    let double = matches!(block, Block::TallGrass | Block::LargeFern | Block::Sunflower);
    can_survive(view, [x, y, z], block) && (!double || view.block(x, y + 1, z) == Block::Air)
}

/// Whether `block` survives at `at`: plants on soil, dry plants on sand and terracotta too.
pub fn can_survive(view: &View, [x, y, z]: [i32; 3], block: Block) -> bool {
    let below = view.block(x, y - 1, z);
    match block {
        Block::ShortDryGrass | Block::TallDryGrass => is_dry_soil(below),
        Block::TallGrassTop | Block::LargeFernTop | Block::SunflowerTop => true,
        _ if block.is_plant() && !block.is_waterlogged() => is_soil(below),
        _ => true,
    }
}
