//! Stronghold corridors: straight (1480a6f90), prison hall (1480a4b10), turns (14809ef60,
//! 14809f220), stairs (14809d0f0, 1480a76b0), chest corridor (14809d650), filler (14809dbe0).

use super::common::{Dir, NORTH_OR_EAST, air_box, fill, selector_box, small_door, torch};
use super::{Door, Kind, Piece};
use crate::block::Block;
use crate::random::Mt19937;
use crate::structure::canvas::Canvas;
use crate::structure::scattered::{create_chest, stairs};

pub fn post_process(p: &Piece, canvas: &mut Canvas, random: &mut Mt19937) {
    let f = &p.frame;
    let north_or_east = NORTH_OR_EAST.contains(&f.orientation.unwrap_or(0));
    match p.kind {
        Kind::StairsDown => {
            selector_box(f, canvas, random, (0, 0, 0), (4, 10, 4));
            small_door(f, canvas, p.door, (1, 7, 0));
            small_door(f, canvas, Door::Opening, (1, 1, 4));
            let steps = [
                (Block::StoneBricks, 2, 6, 1),
                (Block::StoneBricks, 1, 5, 1),
                (Block::StoneBrickSlab, 1, 6, 1),
                (Block::StoneBricks, 1, 5, 2),
                (Block::StoneBricks, 1, 4, 3),
                (Block::StoneBrickSlab, 1, 5, 3),
                (Block::StoneBricks, 2, 4, 3),
                (Block::StoneBricks, 3, 3, 3),
                (Block::StoneBrickSlab, 3, 4, 3),
                (Block::StoneBricks, 3, 3, 2),
                (Block::StoneBricks, 3, 2, 1),
                (Block::StoneBrickSlab, 3, 3, 1),
                (Block::StoneBricks, 2, 2, 1),
                (Block::StoneBricks, 1, 1, 1),
                (Block::StoneBrickSlab, 1, 2, 1),
                (Block::StoneBricks, 1, 1, 2),
                (Block::StoneBrickSlab, 1, 1, 3),
            ];
            steps.iter().for_each(|&(block, x, y, z)| f.place(canvas, block, x, y, z));
        }
        Kind::Straight => {
            selector_box(f, canvas, random, (0, 0, 0), (4, 4, 6));
            small_door(f, canvas, p.door, (1, 1, 0));
            small_door(f, canvas, Door::Opening, (1, 1, 6));
            let (east, west) = (torch(f, Dir::East), torch(f, Dir::West));
            for (block, x, z) in [(east, 1, 1), (west, 3, 1), (east, 1, 5), (west, 3, 5)] {
                if random.next_float() < 0.1 {
                    f.place(canvas, block, x, 2, z);
                }
            }
            if p.flags[0] {
                air_box(f, canvas, (0, 1, 2), (0, 3, 4));
            }
            if p.flags[1] {
                air_box(f, canvas, (4, 1, 2), (4, 3, 4));
            }
        }
        Kind::PrisonHall => {
            selector_box(f, canvas, random, (0, 0, 0), (8, 4, 10));
            small_door(f, canvas, p.door, (1, 1, 0));
            air_box(f, canvas, (1, 1, 10), (3, 3, 10));
            for z in [1, 3, 7, 9] {
                selector_box(f, canvas, random, (4, 1, z), (4, 3, z));
            }
            fill(f, canvas, (4, 1, 4), (4, 3, 6), Block::IronBars0);
            fill(f, canvas, (5, 1, 5), (7, 3, 5), Block::IronBars0);
            f.place(canvas, Block::IronBars0, 4, 3, 2);
            f.place(canvas, Block::IronBars0, 4, 3, 8);
        }
        Kind::LeftTurn | Kind::RightTurn => {
            selector_box(f, canvas, random, (0, 0, 0), (4, 4, 4));
            small_door(f, canvas, p.door, (1, 1, 0));
            let x = if north_or_east == (p.kind == Kind::LeftTurn) { 0 } else { 4 };
            air_box(f, canvas, (x, 1, 1), (x, 3, 3));
        }
        Kind::StraightStairsDown => {
            selector_box(f, canvas, random, (0, 0, 0), (4, 10, 7));
            small_door(f, canvas, p.door, (1, 7, 0));
            small_door(f, canvas, Door::Opening, (1, 1, 7));
            let step = stairs(f, [Block::StoneStairsEast, Block::StoneStairsWest, Block::StoneStairsSouth, Block::StoneStairsNorth], 2);
            for i in 0..6 {
                (1..=3).for_each(|x| f.place(canvas, step, x, 6 - i, 1 + i));
                if i < 5 {
                    (1..=3).for_each(|x| f.place(canvas, Block::StoneBricks, x, 5 - i, 1 + i));
                }
            }
        }
        Kind::ChestCorridor => {
            selector_box(f, canvas, random, (0, 0, 0), (4, 4, 6));
            small_door(f, canvas, p.door, (1, 1, 0));
            small_door(f, canvas, Door::Opening, (1, 1, 6));
            fill(f, canvas, (3, 1, 2), (3, 1, 4), Block::StoneBricks);
            for (x, y, z) in [(3, 1, 1), (3, 1, 5), (3, 2, 2), (3, 2, 4), (2, 1, 2), (2, 1, 3), (2, 1, 4)] {
                f.place(canvas, Block::StoneBrickSlab, x, y, z);
            }
            create_chest(f, canvas, random, (3, 2, 3), Block::ChestWest, "loot_tables/chests/stronghold_corridor.json");
        }
        Kind::Filler => {
            for z in 0..p.value {
                for y in 0..=4 {
                    for x in 0..=4 {
                        let wall = y == 0 || y == 4 || x == 0 || x == 4;
                        f.place(canvas, if wall { Block::StoneBricks } else { Block::Air }, x, y, z);
                    }
                }
            }
        }
        _ => unreachable!("rooms are elsewhere"),
    }
}
