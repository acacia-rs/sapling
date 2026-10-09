//! The village piles (`hay_pile_feature`, `melon_pile_feature`, `pumpkin_pile_feature`,
//! `snow_pile_feature`, `ice_pile_feature`: one class, place 14827e1e0, block 14827ea50), Java's
//! `BlockPileFeature` on BDS's generator.

use super::spec::PileBlock;
use super::view::View;
use crate::block::Block;
use crate::random::Mt19937;
use crate::structure::template::{FULL_CUBE, flags, named};
use crate::terrain::MIN_Y;

/// An ellipse of 5..7 by 5..7 cells over two layers (x, then y, then z), each cell kept when its
/// distance squared is at most `10 f - 6 f`, else at 3.1%; placed on air over a full block or a path.
pub fn pile(view: &mut View, random: &mut Mt19937, [x, y, z]: [i32; 3], block: PileBlock) -> bool {
    if y < MIN_Y + 5 {
        return false;
    }
    let rx = random.next_int(2) + 2;
    let rz = random.next_int(2) + 2;
    for px in x - rx..=x + rx {
        for py in y..=y + 1 {
            for pz in z - rz..=z + rz {
                let (dx, dz) = (x - px, z - pz);
                let ten = random.next_float() * 10.0;
                let six = random.next_float() * 6.0;
                let kept = (dx * dx + dz * dz) as f32 <= ten - six || random.next_float() < 0.031;
                if kept && supports(view.block(px, py - 1, pz)) && view.block(px, py, pz) == Block::Air {
                    let placed = state(block, random);
                    view.set_block(px, py, pz, placed);
                }
            }
        }
    }
    true
}

/// A full block top (material and `BlockType` +0x130 bit 0x40000) or a path (list 0x14ccd3208).
fn supports(below: Block) -> bool {
    if below.name() == "grass_path" {
        return true;
    }
    match flags(below) {
        0 => below.is_solid() && !below.is_leaves() && !below.is_snow_layer(),
        f => f & FULL_CUBE != 0,
    }
}

/// Hay on a random y or x axis, packed ice 1 in 7 blue, pumpkins 5% carved.
fn state(block: PileBlock, random: &mut Mt19937) -> Block {
    match block {
        PileBlock::Hay => named("hay_block", if random.next_int(2) == 0 { "deprecated=0,pillar_axis=y" } else { "deprecated=0,pillar_axis=x" }, false),
        PileBlock::Ice => {
            if random.next_int(7) == 0 {
                Block::BlueIce
            } else {
                Block::PackedIce
            }
        }
        PileBlock::Pumpkin => {
            if random.next_float() >= 0.95 {
                named("carved_pumpkin", "minecraft:cardinal_direction=south", false)
            } else {
                Block::PumpkinSouth
            }
        }
        PileBlock::Melon => Block::MelonBlock,
        PileBlock::Snow => Block::Snow,
    }
}
