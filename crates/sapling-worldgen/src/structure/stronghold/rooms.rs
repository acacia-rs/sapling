//! Stronghold rooms: room crossing (1480a50f0), five-way crossing (14809e1f0), library
//! (14809f4f0).

use super::common::{Dir, air_box, fill, ladder, selector_box, small_door, torch};
use super::{Kind, Piece};
use crate::block::Block;
use crate::random::Mt19937;
use crate::structure::canvas::Canvas;
use crate::structure::piece::Frame;
use crate::structure::scattered::create_chest;

pub fn post_process(p: &Piece, canvas: &mut Canvas, random: &mut Mt19937) {
    match p.kind {
        Kind::RoomCrossing => room_crossing(p, canvas, random),
        Kind::FiveCrossing => five_crossing(p, canvas, random),
        _ => library(p, canvas, random),
    }
}

fn room_crossing(p: &Piece, canvas: &mut Canvas, random: &mut Mt19937) {
    let f = &p.frame;
    selector_box(f, canvas, random, (0, 0, 0), (10, 6, 10));
    small_door(f, canvas, p.door, (4, 1, 0));
    air_box(f, canvas, (4, 1, 10), (6, 3, 10));
    air_box(f, canvas, (0, 1, 4), (0, 3, 6));
    air_box(f, canvas, (10, 1, 4), (10, 3, 6));
    match p.value {
        0 => {
            (1..=3).for_each(|y| f.place(canvas, Block::StoneBricks, 5, y, 5));
            f.place(canvas, torch(f, Dir::West), 4, 3, 5);
            f.place(canvas, torch(f, Dir::East), 6, 3, 5);
            f.place(canvas, torch(f, Dir::South), 5, 3, 4);
            f.place(canvas, torch(f, Dir::North), 5, 3, 6);
            for (x, z) in [(4, 4), (4, 5), (4, 6), (6, 4), (6, 5), (6, 6), (5, 4), (5, 6)] {
                f.place(canvas, Block::SmoothStoneSlab, x, 1, z);
            }
        }
        1 => {
            for i in 0..5 {
                f.place(canvas, Block::StoneBricks, 3, 1, 3 + i);
                f.place(canvas, Block::StoneBricks, 7, 1, 3 + i);
                f.place(canvas, Block::StoneBricks, 3 + i, 1, 3);
                f.place(canvas, Block::StoneBricks, 3 + i, 1, 7);
            }
            (1..=3).for_each(|y| f.place(canvas, Block::StoneBricks, 5, y, 5));
            f.place(canvas, Block::Water, 5, 4, 5);
        }
        2 => storeroom(f, canvas, random),
        _ => {}
    }
}

fn storeroom(f: &Frame, canvas: &mut Canvas, random: &mut Mt19937) {
    let cobble = Block::Cobblestone;
    for i in 1..=9 {
        f.place(canvas, cobble, 1, 3, i);
        f.place(canvas, cobble, 9, 3, i);
    }
    for i in 1..=9 {
        f.place(canvas, cobble, i, 3, 1);
        f.place(canvas, cobble, i, 3, 9);
    }
    for (x, y, z) in [(5, 1, 4), (5, 1, 6), (5, 3, 4), (5, 3, 6), (4, 1, 5), (6, 1, 5), (4, 3, 5), (6, 3, 5)] {
        f.place(canvas, cobble, x, y, z);
    }
    for y in 1..=3 {
        for (x, z) in [(4, 4), (6, 4), (4, 6), (6, 6)] {
            f.place(canvas, cobble, x, y, z);
        }
    }
    // BDS places the torch block's default state, facing "unknown".
    f.place(canvas, Block::TorchUnknown, 5, 3, 5);
    for z in 2..=8 {
        let xs: &[i32] = if (4..=6).contains(&z) { &[2, 3, 7, 8] } else { &[2, 3, 4, 5, 6, 7, 8] };
        xs.iter().for_each(|&x| f.place(canvas, Block::OakPlanks, x, 3, z));
    }
    let step = ladder(f, 4);
    (1..=3).for_each(|y| f.place(canvas, step, 9, y, 3));
    create_chest(f, canvas, random, (3, 4, 8), Block::ChestWest);
}

fn five_crossing(p: &Piece, canvas: &mut Canvas, random: &mut Mt19937) {
    let f = &p.frame;
    let [left_low, left_high, right_low, right_high] = p.flags;
    selector_box(f, canvas, random, (0, 0, 0), (9, 8, 10));
    small_door(f, canvas, p.door, (4, 3, 0));
    if left_low {
        air_box(f, canvas, (0, 3, 1), (0, 5, 3));
    }
    if right_low {
        air_box(f, canvas, (9, 3, 1), (9, 5, 3));
    }
    if left_high {
        air_box(f, canvas, (0, 5, 7), (0, 7, 9));
    }
    if right_high {
        air_box(f, canvas, (9, 5, 7), (9, 7, 9));
    }
    air_box(f, canvas, (5, 1, 10), (7, 3, 10));
    selector_box(f, canvas, random, (1, 2, 1), (8, 2, 6));
    selector_box(f, canvas, random, (4, 1, 5), (4, 4, 9));
    selector_box(f, canvas, random, (8, 1, 5), (8, 4, 9));
    selector_box(f, canvas, random, (1, 4, 7), (3, 4, 9));
    selector_box(f, canvas, random, (1, 3, 5), (3, 3, 6));
    fill(f, canvas, (1, 3, 4), (3, 3, 4), Block::StoneBrickSlab);
    fill(f, canvas, (1, 4, 6), (3, 4, 6), Block::StoneBrickSlab);
    selector_box(f, canvas, random, (5, 1, 7), (7, 1, 8));
    fill(f, canvas, (5, 1, 9), (7, 1, 9), Block::StoneBrickSlab);
    fill(f, canvas, (5, 2, 7), (7, 2, 7), Block::StoneBrickSlab);
    fill(f, canvas, (4, 5, 7), (4, 5, 9), Block::StoneBrickSlab);
    fill(f, canvas, (8, 5, 7), (8, 5, 9), Block::StoneBrickSlab);
    fill(f, canvas, (5, 5, 7), (7, 5, 9), Block::SmoothStoneDoubleSlab);
    f.place(canvas, torch(f, Dir::South), 6, 5, 6);
}

fn library(p: &Piece, canvas: &mut Canvas, random: &mut Mt19937) {
    let f = &p.frame;
    let tall = p.flags[0];
    selector_box(f, canvas, random, (0, 0, 0), (13, if tall { 10 } else { 5 }, 14));
    small_door(f, canvas, p.door, (4, 1, 0));
    f.generate_maybe_box(canvas, random, 0.07, (2, 1, 1), (11, 4, 13), (Block::Web, Block::Web), false, false);
    for z in 1..=13 {
        let shelf = if (z - 1) % 4 == 0 { Block::OakPlanks } else { Block::Bookshelf };
        fill(f, canvas, (1, 1, z), (1, 4, z), shelf);
        fill(f, canvas, (12, 1, z), (12, 4, z), shelf);
        if shelf == Block::OakPlanks {
            f.place(canvas, torch(f, Dir::East), 2, 3, z);
            f.place(canvas, torch(f, Dir::West), 11, 3, z);
        }
        if tall {
            fill(f, canvas, (1, 6, z), (1, 9, z), shelf);
            fill(f, canvas, (12, 6, z), (12, 9, z), shelf);
        }
    }
    for z in (3..12).step_by(2) {
        for x in [3, 6, 9] {
            fill(f, canvas, (x, 1, z), (x + 1, 3, z), Block::Bookshelf);
        }
    }
    if tall {
        gallery(f, canvas);
    }
    create_chest(f, canvas, random, (3, 3, 5), Block::ChestWest);
    if tall {
        create_chest(f, canvas, random, (12, 8, 1), Block::ChestNorth);
    }
}

/// The tall library's balcony, ladder and chandelier.
fn gallery(f: &Frame, canvas: &mut Canvas) {
    let (planks, fence) = (Block::OakPlanks, Block::OakFence0);
    fill(f, canvas, (1, 5, 1), (3, 5, 13), planks);
    fill(f, canvas, (10, 5, 1), (12, 5, 13), planks);
    fill(f, canvas, (4, 5, 1), (9, 5, 2), planks);
    fill(f, canvas, (4, 5, 12), (9, 5, 13), planks);
    [(9, 11), (8, 11), (9, 10)].iter().for_each(|&(x, z)| f.place(canvas, planks, x, 5, z));
    fill(f, canvas, (3, 6, 2), (3, 6, 12), fence);
    fill(f, canvas, (10, 6, 2), (10, 6, 10), fence);
    fill(f, canvas, (4, 6, 2), (9, 6, 2), fence);
    fill(f, canvas, (4, 6, 12), (8, 6, 12), fence);
    [(9, 11), (8, 11), (9, 10)].iter().for_each(|&(x, z)| f.place(canvas, fence, x, 6, z));
    let step = ladder(f, 3);
    (1..=7).for_each(|y| f.place(canvas, step, 10, y, 13));
    let chandelier =
        [(6, 9, 7), (7, 9, 7), (6, 8, 7), (7, 8, 7), (6, 7, 7), (7, 7, 7), (5, 7, 7), (8, 7, 7), (6, 7, 6), (6, 7, 8), (7, 7, 6), (7, 7, 8)];
    chandelier.iter().for_each(|&(x, y, z)| f.place(canvas, fence, x, y, z));
    for (x, z) in [(5, 7), (8, 7), (6, 6), (6, 8), (7, 6), (7, 8)] {
        f.place(canvas, Block::TorchTop, x, 8, z);
    }
}
