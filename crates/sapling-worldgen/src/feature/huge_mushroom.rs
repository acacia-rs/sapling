//! `HugeMushroomFeature` (place 143b7ef50, cap and stem 143b7f230, soil 143b7fc40): a brown or
//! red huge mushroom; dark forests and swamps pick it among their trees.

use super::view::View;
use crate::block::Block;
use crate::random::Mt19937;
use crate::terrain::{HEIGHT, MIN_Y};

const BROWN: [Block; 9] = [
    Block::BrownMushroomBlock1,
    Block::BrownMushroomBlock2,
    Block::BrownMushroomBlock3,
    Block::BrownMushroomBlock4,
    Block::BrownMushroomBlock5,
    Block::BrownMushroomBlock6,
    Block::BrownMushroomBlock7,
    Block::BrownMushroomBlock8,
    Block::BrownMushroomBlock9,
];
const RED: [Block; 9] = [
    Block::RedMushroomBlock1,
    Block::RedMushroomBlock2,
    Block::RedMushroomBlock3,
    Block::RedMushroomBlock4,
    Block::RedMushroomBlock5,
    Block::RedMushroomBlock6,
    Block::RedMushroomBlock7,
    Block::RedMushroomBlock8,
    Block::RedMushroomBlock9,
];

/// Swamp and swamp hills: the biome 141603120 compares against (the biome registry's +0x308).
pub fn is_swamp(biome: u32) -> bool {
    matches!(biome, 6 | 134)
}

fn is_soil(block: Block) -> bool {
    matches!(block, Block::Dirt | Block::CoarseDirt | Block::Mycelium | Block::Podzol | Block::GrassBlock)
}

/// `BlockSource::isSolidBlockingBlock`: caps and stems never replace one.
fn solid_blocking(block: Block) -> bool {
    block.blocks_motion() && !block.is_leaves()
}

/// Type (`nextInt(2)`: brown, red), height `nextInt(3) + 4`, doubled one time in 12; the space
/// above (radius 0 up to 3 over the base, 3 beyond) must be air or leaves.
pub fn place(view: &mut View, random: &mut Mt19937, [x, y, z]: [i32; 3], swamp: bool) -> bool {
    let red = random.next_int(2) == 1;
    let base = random.next_int(3) + 4;
    let height = base << i32::from(random.next_int(12) == 0);
    let top_limit = MIN_Y + HEIGHT as i32;
    if y <= MIN_Y || y + height + 1 >= top_limit {
        return false;
    }
    for cy in y..y + height + 2 {
        let r = if cy > y + 3 { 3 } else { 0 };
        for cx in x - r..=x + r {
            for cz in z - r..=z + r {
                let block = view.block(cx, cy, cz);
                if block == Block::Air || block.is_leaves() {
                    continue;
                }
                let origin_mushroom = [cx, cy, cz] == [x, y, z] && matches!(block, Block::BrownMushroom | Block::RedMushroom);
                if !origin_mushroom {
                    return false;
                }
            }
        }
    }
    let top = y + height;
    // 143b7f230: off soil, in a swamp (141603120's biome), the stem may root through one or two water blocks.
    let sink = if is_soil(view.block(x, y - 1, z)) {
        0
    } else if swamp && view.block(x, y - 1, z) == Block::Water && is_soil(view.block(x, y - 2, z)) {
        1
    } else if swamp && view.block(x, y - 1, z) == Block::Water && view.block(x, y - 2, z) == Block::Water && is_soil(view.block(x, y - 3, z)) {
        2
    } else {
        return false;
    };
    let y = y - sink;
    let caps = if red { &RED } else { &BROWN };
    for cy in (if red { top - 3 } else { top })..=top {
        let r = if !red { 3 } else if cy < top { 2 } else { 1 };
        let restricted = !red || cy < top;
        let (x0, x1, z0, z1) = (x - r, x + r, z - r, z + r);
        for cx in x0..=x1 {
            for cz in z0..=z1 {
                let mut bits = 5 + i32::from(cx == x1) - i32::from(cx == x0) + 3 * (i32::from(cz == z1) - i32::from(cz == z0));
                if restricted {
                    if (cx == x0 || cx == x1) && (cz == z0 || cz == z1) {
                        continue;
                    }
                    for (bx, bz, value) in [
                        (x0 + 1, z0, 1),
                        (x0, z0 + 1, 1),
                        (x1 - 1, z0, 3),
                        (x1, z0 + 1, 3),
                        (x0 + 1, z1, 7),
                        (x0, z1 - 1, 7),
                        (x1 - 1, z1, 9),
                        (x1, z1 - 1, 9),
                    ] {
                        if cx == bx && cz == bz {
                            bits = value;
                        }
                    }
                }
                if bits == 5 && cy < top {
                    continue;
                }
                if !solid_blocking(view.block(cx, cy, cz)) {
                    view.set_block(cx, cy, cz, caps[bits as usize - 1]);
                }
            }
        }
    }
    for cy in y..top {
        if !solid_blocking(view.block(x, cy, z)) {
            view.set_block(x, cy, z, Block::MushroomStem);
        }
    }
    true
}
