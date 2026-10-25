//! The desert pyramid (`DesertPyramidPiece::postProcess` 148360db0): Java's pyramid, its chests
//! facing the room's centre, then the cellar (cellar.rs).

use super::{STAIRS_DATA, cellar, stairs};
use crate::block::Block;
use crate::block_entity::BlockEntity;
use crate::random::Mt19937;
use crate::structure::canvas::Canvas;
use crate::structure::piece::Frame;

const SANDSTONE_STAIRS: [Block; 4] =
    [Block::SandstoneStairsEast, Block::SandstoneStairsWest, Block::SandstoneStairsSouth, Block::SandstoneStairsNorth];
const W: i32 = 21;
const D: i32 = 21;

pub fn post_process(f: &Frame, canvas: &mut Canvas, random: &mut Mt19937) {
    let (air, sandstone, cut, chiseled) = (Block::Air, Block::Sandstone, Block::CutSandstone, Block::ChiseledSandstone);
    let (orange, blue) = (Block::OrangeTerracotta, Block::BlueTerracotta);
    let fill = |canvas: &mut Canvas, from, to, edge, inside| f.generate_box(canvas, from, to, edge, inside, false);
    fill(canvas, (0, -4, 0), (W - 1, 0, D - 1), sandstone, sandstone);
    for i in 1..=9 {
        fill(canvas, (i, i, i), (W - 1 - i, i, D - 1 - i), sandstone, sandstone);
        fill(canvas, (i + 1, i, i + 1), (W - 2 - i, i, D - 2 - i), air, air);
    }
    for x in 0..W {
        for z in 0..D {
            f.fill_column_down(canvas, sandstone, x, -5, z);
        }
    }
    let [east, west, south, north] = STAIRS_DATA.map(|data| stairs(f, SANDSTONE_STAIRS, data));
    for x in [0, W - 5] {
        fill(canvas, (x, 0, 0), (x + 4, 9, 4), sandstone, air);
        fill(canvas, (x + 1, 10, 1), (x + 3, 10, 3), sandstone, sandstone);
        f.place(canvas, north, x + 2, 10, 0);
        f.place(canvas, south, x + 2, 10, 4);
        f.place(canvas, east, x, 10, 2);
        f.place(canvas, west, x + 4, 10, 2);
    }
    fill(canvas, (8, 0, 0), (12, 4, 4), sandstone, air);
    fill(canvas, (9, 1, 0), (11, 3, 4), air, air);
    for (x, y) in [(9, 1), (9, 2), (9, 3), (10, 3), (11, 3), (11, 2), (11, 1)] {
        f.place(canvas, cut, x, y, 1);
    }
    fill(canvas, (4, 1, 1), (8, 3, 3), sandstone, air);
    fill(canvas, (4, 1, 2), (8, 2, 2), air, air);
    fill(canvas, (12, 1, 1), (16, 3, 3), sandstone, air);
    fill(canvas, (12, 1, 2), (16, 2, 2), air, air);
    fill(canvas, (5, 4, 5), (W - 6, 4, D - 6), sandstone, sandstone);
    fill(canvas, (9, 4, 9), (11, 4, 11), air, air);
    for (x, z) in [(8, 8), (12, 8), (8, 12), (12, 12)] {
        fill(canvas, (x, 1, z), (x, 3, z), cut, sandstone);
    }
    fill(canvas, (1, 1, 5), (4, 4, 11), sandstone, sandstone);
    fill(canvas, (W - 5, 1, 5), (W - 2, 4, 11), sandstone, sandstone);
    fill(canvas, (6, 7, 9), (6, 7, 11), sandstone, sandstone);
    fill(canvas, (W - 7, 7, 9), (W - 7, 7, 11), sandstone, sandstone);
    fill(canvas, (5, 5, 9), (5, 7, 11), cut, cut);
    fill(canvas, (W - 6, 5, 9), (W - 6, 7, 11), cut, cut);
    for (x, y) in [(5, 5), (5, 6), (6, 6), (W - 6, 5), (W - 6, 6), (W - 7, 6)] {
        f.place(canvas, air, x, y, 10);
    }
    fill(canvas, (2, 4, 4), (2, 6, 4), air, air);
    fill(canvas, (W - 3, 4, 4), (W - 3, 6, 4), air, air);
    for x in [2, W - 3] {
        f.place(canvas, north, x, 4, 5);
        f.place(canvas, north, x, 3, 4);
    }
    fill(canvas, (1, 1, 3), (2, 2, 3), sandstone, sandstone);
    fill(canvas, (W - 3, 1, 3), (W - 2, 2, 3), sandstone, sandstone);
    // BDS places unturned stairs where Java has sandstone.
    f.place(canvas, Block::SandstoneStairsEast, 1, 1, 2);
    f.place(canvas, Block::SandstoneStairsEast, W - 2, 1, 2);
    f.place(canvas, Block::SandstoneSlab, 1, 2, 2);
    f.place(canvas, Block::SandstoneSlab, W - 2, 2, 2);
    f.place(canvas, west, 2, 1, 2);
    f.place(canvas, east, W - 3, 1, 2);
    // BDS's west ledge runs one block further than the east one.
    fill(canvas, (4, 3, 5), (4, 3, 18), sandstone, sandstone);
    fill(canvas, (W - 5, 3, 5), (W - 5, 3, 17), sandstone, sandstone);
    fill(canvas, (3, 1, 5), (4, 2, 16), air, air);
    fill(canvas, (W - 6, 1, 5), (W - 5, 2, 16), air, air);
    for z in (5..=17).step_by(2) {
        f.place(canvas, cut, 4, 1, z);
        f.place(canvas, chiseled, 4, 2, z);
        f.place(canvas, cut, W - 5, 1, z);
        f.place(canvas, chiseled, W - 5, 2, z);
    }
    for (x, z) in [(10, 7), (10, 8), (9, 9), (11, 9), (8, 10), (12, 10), (7, 10), (13, 10), (9, 11), (11, 11), (10, 12), (10, 13)] {
        f.place(canvas, orange, x, 0, z);
    }
    f.place(canvas, blue, 10, 0, 10);
    // Each tower's front: columns of three, by height.
    let rows: [[Block; 3]; 7] = [
        [cut, orange, cut],
        [cut, orange, cut],
        [orange, chiseled, orange],
        [cut, orange, cut],
        [orange, chiseled, orange],
        [orange, orange, orange],
        [cut, cut, cut],
    ];
    for x in [0, W - 1] {
        for (row, y) in rows.iter().zip(2..) {
            for (block, z) in row.iter().zip(1..) {
                f.place(canvas, *block, x, y, z);
            }
        }
    }
    for x in [2, W - 3] {
        for (row, y) in rows.iter().zip(2..) {
            for (block, dx) in row.iter().zip(-1..) {
                f.place(canvas, *block, x + dx, y, 0);
            }
        }
    }
    fill(canvas, (8, 4, 0), (12, 6, 0), cut, cut);
    f.place(canvas, air, 8, 6, 0);
    f.place(canvas, air, 12, 6, 0);
    f.place(canvas, orange, 9, 5, 0);
    f.place(canvas, chiseled, 10, 5, 0);
    f.place(canvas, orange, 11, 5, 0);
    treasure_room(f, canvas, random);
    cellar::place(f, canvas, random);
}

/// The shaft under the floor's centre, the trapped room and its four chests.
fn treasure_room(f: &Frame, canvas: &mut Canvas, random: &mut Mt19937) {
    let (air, sandstone, cut, chiseled) = (Block::Air, Block::Sandstone, Block::CutSandstone, Block::ChiseledSandstone);
    f.generate_box(canvas, (8, -14, 8), (12, -11, 12), cut, cut, false);
    f.generate_box(canvas, (8, -10, 8), (12, -10, 12), chiseled, chiseled, false);
    f.generate_box(canvas, (8, -9, 8), (12, -9, 12), cut, cut, false);
    f.generate_box(canvas, (8, -8, 8), (12, -1, 12), sandstone, sandstone, false);
    f.generate_box(canvas, (9, -11, 9), (11, -1, 11), air, air, false);
    f.place(canvas, Block::StonePressurePlate, 10, -11, 10);
    f.generate_box(canvas, (9, -13, 9), (11, -13, 11), Block::Tnt, air, false);
    for (x, z, dx, dz) in [(8, 10, -1, 0), (12, 10, 1, 0), (10, 8, 0, -1), (10, 12, 0, 1)] {
        f.place(canvas, air, x, -11, z);
        f.place(canvas, air, x, -10, z);
        f.place(canvas, chiseled, x + dx, -10, z + dz);
        f.place(canvas, cut, x + dx, -11, z + dz);
    }
    for (x, z, chest) in [(10, 12, Block::ChestNorth),(8, 10, Block::ChestEast), (10, 8, Block::ChestSouth), (12, 10, Block::ChestWest)] {
        create_chest(f, canvas, random, (x, -11, z), chest, "loot_tables/chests/desert_pyramid.json");
    }
}

/// `createChest` (141f49ef0): a chest set directly where none is, then one draw for its loot `table`'s seed.
pub fn create_chest(f: &Frame, canvas: &mut Canvas, random: &mut Mt19937, (x, y, z): (i32, i32, i32), chest: Block, table: &'static str) {
    let (wx, wy, wz) = f.world(x, y, z);
    if canvas.bb.contains(wx, wy, wz) && !Block::CHESTS.contains(&canvas.get(wx, wy, wz)) {
        canvas.set(wx, wy, wz, chest);
        canvas.set_entity(wx, wy, wz, BlockEntity::Loot { table, seed: random.next_int_unbounded() });
    }
}
