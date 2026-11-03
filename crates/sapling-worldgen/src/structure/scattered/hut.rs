//! The swamp hut (`SwamplandHut::postProcess` 14836de70): Java's hut one block taller, walls up
//! to y 4 and the roof at 5.

use super::{STAIRS_DATA, stairs};
use crate::block::Block;
use crate::block_entity::BlockEntity;
use crate::random::Mt19937;
use crate::structure::canvas::Canvas;
use crate::structure::piece::Frame;

const SPRUCE_STAIRS: [Block; 4] = [Block::SpruceStairsEast, Block::SpruceStairsWest, Block::SpruceStairsSouth, Block::SpruceStairsNorth];
const CAULDRONS: [Block; 3] = [Block::Cauldron2, Block::Cauldron4, Block::Cauldron6];
/// The hut cauldron's potion by its draw of 100 (14836e76b..e7b5): below each bound, that potion id;
/// water breathing, fire resistance, swiftness, slowness, poison, weakness, else healing.
const POTIONS: [(i32, i16); 6] = [(10, 19), (15, 12), (30, 14), (40, 17), (65, 25), (75, 34)];
const HEALING: i16 = 21;

fn potion(draw: i32) -> i16 {
    POTIONS.iter().find(|(below, _)| draw < *below).map_or(HEALING, |(_, id)| *id)
}

pub fn post_process(f: &Frame, canvas: &mut Canvas, random: &mut Mt19937) {
    let (air, planks, log, fence) = (Block::Air, Block::SprucePlanks, Block::OakLog, Block::OakFence0);
    f.generate_box(canvas, (0, 2, 0), (6, 3, 8), air, air, true);
    for (from, to) in [
        ((1, 1, 1), (5, 1, 7)),
        ((1, 5, 2), (5, 5, 7)),
        ((2, 1, 0), (4, 1, 0)),
        ((2, 2, 2), (3, 4, 2)),
        ((1, 2, 3), (1, 4, 6)),
        ((5, 2, 3), (5, 4, 6)),
        ((2, 2, 7), (4, 4, 7)),
    ] {
        f.generate_box(canvas, from, to, planks, planks, false);
    }
    for (x, z) in [(1, 2), (5, 2), (1, 7), (5, 7)] {
        f.generate_box(canvas, (x, 0, z), (x, 4, z), log, log, false);
    }
    f.place(canvas, fence, 2, 3, 2);
    f.place(canvas, fence, 3, 3, 7);
    f.place(canvas, air, 1, 3, 4);
    f.place(canvas, air, 5, 3, 4);
    f.place(canvas, air, 5, 3, 5);
    f.place(canvas, Block::FlowerPot, 1, 3, 5);
    // BDS's pot holds a red mushroom (oracle 20261003-structures).
    let (x, y, z) = f.world(1, 3, 5);
    if canvas.bb.contains(x, y, z) && canvas.get(x, y, z) == Block::FlowerPot {
        canvas.set_entity(x, y, z, BlockEntity::FlowerPot("minecraft:red_mushroom"));
    }
    // The taller wall's lintel over the door.
    f.place(canvas, planks, 4, 4, 2);
    f.place(canvas, Block::CraftingTable, 3, 2, 6);
    // The cauldron is set directly, its level and potion drawn only when it is.
    let (x, y, z) = f.world(4, 2, 6);
    if canvas.bb.contains(x, y, z) && !CAULDRONS.contains(&canvas.get(x, y, z)) {
        canvas.set(x, y, z, CAULDRONS[random.next_int(3) as usize]);
        canvas.set_entity(x, y, z, BlockEntity::Cauldron(potion(random.next_int(100))));
    }
    f.place(canvas, fence, 1, 2, 1);
    f.place(canvas, fence, 5, 2, 1);
    let [east, west, south, north] = STAIRS_DATA.map(|data| stairs(f, SPRUCE_STAIRS, data));
    f.generate_box(canvas, (0, 5, 1), (6, 5, 1), north, north, false);
    f.generate_box(canvas, (0, 5, 2), (0, 5, 7), east, east, false);
    f.generate_box(canvas, (6, 5, 2), (6, 5, 7), west, west, false);
    f.generate_box(canvas, (0, 5, 8), (6, 5, 8), south, south, false);
    for (x, z) in [(1, 2), (5, 2), (1, 7), (5, 7)] {
        f.fill_column_down(canvas, log, x, -1, z);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_cauldrons_potion_follows_bds_bounds() {
        let drawn: Vec<i16> = [0, 9, 10, 14, 15, 29, 30, 39, 40, 64, 65, 74, 75, 99].map(super::potion).to_vec();
        assert_eq!(drawn, [19, 19, 12, 12, 14, 14, 17, 17, 25, 25, 34, 34, 21, 21]);
    }
}
