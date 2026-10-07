//! The solid block `_prepareHeights` (14822eb60) writes: the deepslate gradient
//! (`DepthBasedBlockSupplier`, "minecraft:depth_based_layer") under Java 1.18.1's `OreVeinifier`.

use super::Density;
use crate::block::Block;

struct VeinType {
    min_y: i32,
    max_y: i32,
    ore: Block,
    raw: Block,
    filler: Block,
}

const COPPER: VeinType = VeinType { min_y: 0, max_y: 50, ore: Block::CopperOre, raw: Block::RawCopperBlock, filler: Block::Granite };
const IRON: VeinType = VeinType { min_y: -60, max_y: -8, ore: Block::DeepslateIronOre, raw: Block::RawIronBlock, filler: Block::Tuff };

impl Density {
    /// `veins` are the interpolated veininess, vein A and vein B at the block.
    pub fn solid_block(&self, [x, y, z]: [i32; 3], [veininess, vein_a, vein_b]: [f32; 3]) -> Block {
        let base = self.deepslate_or_stone(x, y, z);
        let vein = if veininess > 0.0 { &COPPER } else { &IRON };
        let (above_min, below_max) = ((y - vein.min_y) as f32, (vein.max_y - y) as f32);
        if !(0.0 <= above_min) || !(0.0 <= below_max) {
            return base;
        }
        let edge = above_min.min(below_max) / 20.0;
        let edge_falloff = if 1.0 < edge { 0.0 } else { edge * 0.2 + -0.2 };
        let strength = veininess.abs();
        if edge_falloff + strength < 0.4 {
            return base;
        }
        let mut random = self.veins.ore.at(x, y, z);
        if !(random.next_float() <= 0.7) {
            return base;
        }
        if !((vein_a.abs() + -0.08).max(vein_b.abs() + -0.08) < 0.0) {
            return base;
        }
        let t = (strength + -0.4) / 0.200_000_02;
        let chance = if !(0.0 <= t) { 0.1 } else if t <= 1.0 { t * 0.200_000_02 + 0.1 } else { 0.3 };
        if !(random.next_float() < chance) {
            return vein.filler;
        }
        let gap = self.veins.gap.sample(x as f32, y as f32, z as f32);
        if !(1.0 < gap || -0.3 < gap.max(-1.0)) {
            return vein.filler;
        }
        if random.next_float() < 0.02 { vein.raw } else { vein.ore }
    }

    /// Deepslate below y 0, stone from y 8, a positional coin in between.
    fn deepslate_or_stone(&self, x: i32, y: i32, z: i32) -> Block {
        if y < 0 {
            return Block::Deepslate;
        }
        if y >= 9 {
            return Block::Stone;
        }
        let t = y as f32 * 0.125;
        let chance = if 1.0 < t { 0.0 } else { 1.0 - t };
        if chance <= self.veins.deepslate.at(x, y, z).next_float() { Block::Stone } else { Block::Deepslate }
    }
}
