//! The jungle temple (`JunglePyramidPiece::postProcess` 148366d30): Java's temple; its stone
//! boxes draw cobblestone or mossy cobblestone per cell (selector 148321b40, `nextFloat() < 0.4`).
//! Trap blocks carry the states BDS saves for the south-facing piece.

use super::desert::create_chest;
use super::{STAIRS_DATA, stairs};
use crate::block::Block;
use crate::random::Mt19937;
use crate::structure::canvas::Canvas;
use crate::structure::piece::Frame;

const STONE_STAIRS: [Block; 4] = [Block::StoneStairsEast, Block::StoneStairsWest, Block::StoneStairsSouth, Block::StoneStairsNorth];

type Cell = (i32, i32, i32);

/// `generateBox` with a block selector (141f5a6c0): y, x, z, one draw per cell.
fn stone(f: &Frame, canvas: &mut Canvas, random: &mut Mt19937, (x0, y0, z0): Cell, (x1, y1, z1): Cell) {
    for y in y0..=y1 {
        for x in x0..=x1 {
            for z in z0..=z1 {
                let block = if random.next_float() < 0.4 { Block::Cobblestone } else { Block::MossyCobblestone };
                f.place(canvas, block, x, y, z);
            }
        }
    }
}

fn air(f: &Frame, canvas: &mut Canvas, (x0, y0, z0): Cell, (x1, y1, z1): Cell) {
    f.generate_box(canvas, (x0, y0, z0), (x1, y1, z1), Block::Air, Block::Air, false);
}

pub fn post_process(f: &Frame, canvas: &mut Canvas, random: &mut Mt19937) {
    let boxes: [(Cell, Cell); 13] = [
        ((0, -4, 0), (11, 0, 14)),
        ((2, 1, 2), (9, 2, 2)),
        ((2, 1, 12), (9, 2, 12)),
        ((2, 1, 3), (2, 2, 11)),
        ((9, 1, 3), (9, 2, 11)),
        ((1, 3, 1), (10, 6, 1)),
        ((1, 3, 13), (10, 6, 13)),
        ((1, 3, 2), (1, 6, 12)),
        ((10, 3, 2), (10, 6, 12)),
        ((2, 3, 2), (9, 3, 12)),
        ((2, 6, 2), (9, 6, 12)),
        ((3, 7, 3), (8, 7, 11)),
        ((4, 8, 4), (7, 8, 10)),
    ];
    for (from, to) in boxes {
        stone(f, canvas, random, from, to);
    }
    for (from, to) in [((3, 1, 3), (8, 2, 11)), ((4, 3, 6), (7, 3, 9)), ((2, 4, 2), (9, 5, 12)), ((4, 6, 5), (7, 6, 9)), ((5, 7, 6), (6, 7, 8))] {
        air(f, canvas, from, to);
    }
    for (from, to) in [((5, 1, 2), (6, 2, 2)), ((5, 2, 12), (6, 2, 12)), ((5, 5, 1), (6, 5, 1)), ((5, 5, 13), (6, 5, 13))] {
        air(f, canvas, from, to);
    }
    for (x, z) in [(1, 5), (10, 5), (1, 9), (10, 9)] {
        f.place(canvas, Block::Air, x, 5, z);
    }
    for z in [0, 14] {
        for x in [2, 4, 7, 9] {
            stone(f, canvas, random, (x, 4, z), (x, 5, z));
        }
    }
    stone(f, canvas, random, (5, 6, 0), (6, 6, 0));
    for x in [0, 11] {
        for z in (2..=12).step_by(2) {
            stone(f, canvas, random, (x, 4, z), (x, 5, z));
        }
        stone(f, canvas, random, (x, 6, 5), (x, 6, 5));
        stone(f, canvas, random, (x, 6, 9), (x, 6, 9));
    }
    for (x, z) in [(2, 2), (9, 2), (2, 12), (9, 12)] {
        stone(f, canvas, random, (x, 7, z), (x, 9, z));
    }
    for (x, z) in [(4, 4), (7, 4), (4, 10), (7, 10)] {
        stone(f, canvas, random, (x, 9, z), (x, 9, z));
    }
    stone(f, canvas, random, (5, 9, 7), (6, 9, 7));
    let [east, west, south, north] = STAIRS_DATA.map(|data| stairs(f, STONE_STAIRS, data));
    for (block, x, y, z) in [(north, 5, 9, 6), (north, 6, 9, 6), (south, 5, 9, 8), (south, 6, 9, 8)] {
        f.place(canvas, block, x, y, z);
    }
    for x in 4..=7 {
        f.place(canvas, north, x, 0, 0);
    }
    for x in [4, 7] {
        for i in 0..3 {
            f.place(canvas, north, x, 1 + i, 8 + i);
        }
    }
    stone(f, canvas, random, (4, 1, 9), (4, 1, 9));
    stone(f, canvas, random, (7, 1, 9), (7, 1, 9));
    stone(f, canvas, random, (4, 1, 10), (7, 2, 10));
    stone(f, canvas, random, (5, 4, 5), (6, 4, 5));
    f.place(canvas, east, 4, 4, 5);
    f.place(canvas, west, 7, 4, 5);
    for i in 0..4 {
        f.place(canvas, south, 5, -i, 6 + i);
        f.place(canvas, south, 6, -i, 6 + i);
        air(f, canvas, (5, -i, 7 + i), (6, -i, 9 + i));
    }
    air(f, canvas, (1, -3, 12), (10, -1, 13));
    air(f, canvas, (1, -3, 1), (3, -1, 13));
    air(f, canvas, (1, -3, 1), (9, -1, 5));
    for z in (1..=13).step_by(2) {
        stone(f, canvas, random, (1, -3, z), (1, -2, z));
    }
    for z in (2..=12).step_by(2) {
        stone(f, canvas, random, (1, -1, z), (3, -1, z));
    }
    stone(f, canvas, random, (2, -2, 1), (5, -2, 1));
    stone(f, canvas, random, (7, -2, 1), (9, -2, 1));
    stone(f, canvas, random, (6, -3, 1), (6, -3, 1));
    stone(f, canvas, random, (6, -1, 1), (6, -1, 1));
    traps(f, canvas, random);
}

/// The two tripwire traps, the lever puzzle and their chests.
fn traps(f: &Frame, canvas: &mut Canvas, random: &mut Mt19937) {
    let (mossy, wire) = (Block::MossyCobblestone, Block::RedstoneWire);
    let place = |canvas: &mut Canvas, block: Block, cells: &[Cell]| cells.iter().for_each(|&(x, y, z)| f.place(canvas, block, x, y, z));
    f.place(canvas, Block::TripwireHookEast, 1, -3, 8);
    f.place(canvas, Block::TripwireHookWest, 4, -3, 8);
    place(canvas, Block::TripWireEastWest, &[(2, -3, 8), (3, -3, 8)]);
    place(canvas, wire, &[(5, -3, 7), (5, -3, 6), (5, -3, 5), (5, -3, 4), (5, -3, 3), (5, -3, 2), (5, -3, 1), (4, -3, 1)]);
    f.place(canvas, mossy, 3, -3, 1);
    create_container(f, canvas, random, (3, -2, 1), Block::DispenserSouth);
    f.place(canvas, Block::VineAllSides, 3, -2, 2);
    f.place(canvas, Block::TripwireHookSouth, 7, -3, 1);
    f.place(canvas, Block::TripwireHookNorth, 7, -3, 5);
    place(canvas, Block::TripWireNorthSouth, &[(7, -3, 2), (7, -3, 3), (7, -3, 4)]);
    place(canvas, wire, &[(8, -3, 6), (9, -3, 6), (9, -3, 5)]);
    f.place(canvas, mossy, 9, -3, 4);
    f.place(canvas, wire, 9, -2, 4);
    create_container(f, canvas, random, (9, -2, 3), Block::DispenserWest);
    place(canvas, Block::VineAllSides, &[(8, -1, 3), (8, -2, 3)]);
    create_chest(f, canvas, random, (8, -3, 3), Block::ChestWest);
    place(canvas, mossy, &[(9, -3, 2), (8, -3, 1), (4, -3, 5), (5, -2, 5), (5, -1, 5), (6, -3, 5), (7, -2, 5), (7, -1, 5), (8, -3, 5)]);
    stone(f, canvas, random, (9, -1, 1), (9, -1, 5));
    air(f, canvas, (8, -3, 8), (10, -1, 10));
    place(canvas, Block::ChiseledStoneBricks, &[(8, -2, 11), (9, -2, 11), (10, -2, 11)]);
    place(canvas, Block::LeverSouth, &[(8, -2, 12), (9, -2, 12), (10, -2, 12)]);
    stone(f, canvas, random, (8, -3, 8), (8, -3, 10));
    stone(f, canvas, random, (10, -3, 8), (10, -3, 10));
    f.place(canvas, mossy, 10, -2, 9);
    place(canvas, wire, &[(8, -2, 9), (8, -2, 10), (10, -1, 9)]);
    f.place(canvas, Block::StickyPistonUp, 9, -2, 8);
    place(canvas, Block::StickyPistonWest, &[(10, -2, 8), (10, -1, 8)]);
    f.place(canvas, Block::RepeaterSouth, 10, -2, 10);
    create_chest(f, canvas, random, (9, -3, 10), Block::ChestSouth);
}

/// `createDispenser` (141f4a850): as `createChest`, one draw for the loot when placed.
fn create_container(f: &Frame, canvas: &mut Canvas, random: &mut Mt19937, (x, y, z): Cell, block: Block) {
    let (wx, wy, wz) = f.world(x, y, z);
    if canvas.bb.contains(wx, wy, wz) && !matches!(canvas.get(wx, wy, wz), Block::DispenserSouth | Block::DispenserWest) {
        canvas.set(wx, wy, wz, block);
        random.next_int_unbounded();
    }
}
