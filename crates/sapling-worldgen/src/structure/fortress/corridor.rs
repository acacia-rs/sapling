//! The castle corridors' post-processes: straight, crossing, both turns, stairs and the T balcony
//! (14832ef90, 14832f790, 14832fe40 right, 148330850 left, 1483311f0, 148331a90).

use super::{air, bricks, down, fences};
use crate::block::Block;
use crate::random::Mt19937;
use crate::structure::canvas::Canvas;
use crate::structure::piece::Frame;
use crate::structure::scattered::create_chest;

fn corridor_floor(f: &Frame, c: &mut Canvas) {
    for x in 0..=4 {
        for z in 0..=4 {
            down(f, c, x, z);
        }
    }
}

pub fn small(f: &Frame, c: &mut Canvas) {
    bricks(f, c, (0, 0, 0), (4, 1, 4));
    air(f, c, (0, 2, 0), (4, 5, 4));
    bricks(f, c, (0, 2, 0), (0, 5, 4));
    bricks(f, c, (4, 2, 0), (4, 5, 4));
    fences(f, c, (0, 3, 1), (0, 4, 1));
    fences(f, c, (0, 3, 3), (0, 4, 3));
    fences(f, c, (4, 3, 1), (4, 4, 1));
    fences(f, c, (4, 3, 3), (4, 4, 3));
    bricks(f, c, (0, 6, 0), (4, 6, 4));
    corridor_floor(f, c);
}

pub fn crossing(f: &Frame, c: &mut Canvas) {
    bricks(f, c, (0, 0, 0), (4, 1, 4));
    air(f, c, (0, 2, 0), (4, 5, 4));
    bricks(f, c, (0, 2, 0), (0, 5, 0));
    bricks(f, c, (4, 2, 0), (4, 5, 0));
    bricks(f, c, (0, 2, 4), (0, 5, 4));
    bricks(f, c, (4, 2, 4), (4, 5, 4));
    bricks(f, c, (0, 6, 0), (4, 6, 4));
    corridor_floor(f, c);
}

/// A turn walled on its outer side (x 0 for a right turn, x 4 for a left one), its chest in the corner.
pub fn turn(f: &Frame, c: &mut Canvas, random: &mut Mt19937, chest: bool, left: bool) {
    let (wall, open) = if left { (4, 0) } else { (0, 4) };
    bricks(f, c, (0, 0, 0), (4, 1, 4));
    air(f, c, (0, 2, 0), (4, 5, 4));
    bricks(f, c, (wall, 2, 0), (wall, 5, 4));
    fences(f, c, (wall, 3, 1), (wall, 4, 1));
    fences(f, c, (wall, 3, 3), (wall, 4, 3));
    bricks(f, c, (open, 2, 0), (open, 5, 0));
    if left {
        bricks(f, c, (0, 2, 4), (3, 5, 4));
    } else {
        bricks(f, c, (1, 2, 4), (4, 5, 4));
    }
    fences(f, c, (1, 3, 4), (1, 4, 4));
    fences(f, c, (3, 3, 4), (3, 4, 4));
    if chest {
        create_chest(f, c, random, (if left { 3 } else { 1 }, 2, 3), Block::ChestWest, "loot_tables/chests/nether_bridge.json");
    }
    bricks(f, c, (0, 6, 0), (4, 6, 4));
    corridor_floor(f, c);
}

pub fn stairs(f: &Frame, c: &mut Canvas) {
    let step = super::stairs(f, 2);
    for z in 0..=9 {
        let floor = (7 - z).max(1);
        let ceiling = (floor + 5).max(14 - z).min(13);
        bricks(f, c, (0, 0, z), (4, floor, z));
        air(f, c, (1, floor + 1, z), (3, ceiling - 1, z));
        if z <= 6 {
            for x in 1..=3 {
                f.place(c, step, x, floor + 1, z);
            }
        }
        bricks(f, c, (0, ceiling, z), (4, ceiling, z));
        bricks(f, c, (0, floor + 1, z), (0, ceiling - 1, z));
        bricks(f, c, (4, floor + 1, z), (4, ceiling - 1, z));
        if z & 1 == 0 {
            fences(f, c, (0, floor + 2, z), (0, floor + 3, z));
            fences(f, c, (4, floor + 2, z), (4, floor + 3, z));
        }
        for x in 0..=4 {
            down(f, c, x, z);
        }
    }
}

pub fn balcony(f: &Frame, c: &mut Canvas) {
    bricks(f, c, (0, 0, 0), (8, 1, 8));
    air(f, c, (0, 2, 0), (8, 5, 8));
    bricks(f, c, (0, 6, 0), (8, 6, 5));
    bricks(f, c, (0, 2, 0), (2, 5, 0));
    bricks(f, c, (6, 2, 0), (8, 5, 0));
    fences(f, c, (1, 3, 0), (1, 4, 0));
    fences(f, c, (7, 3, 0), (7, 4, 0));
    bricks(f, c, (0, 2, 4), (8, 2, 8));
    air(f, c, (1, 1, 4), (2, 2, 4));
    air(f, c, (6, 1, 4), (7, 2, 4));
    fences(f, c, (0, 3, 8), (8, 3, 8));
    fences(f, c, (0, 3, 6), (0, 3, 7));
    fences(f, c, (8, 3, 6), (8, 3, 7));
    bricks(f, c, (0, 3, 4), (0, 5, 5));
    bricks(f, c, (8, 3, 4), (8, 5, 5));
    bricks(f, c, (1, 3, 5), (2, 5, 5));
    bricks(f, c, (6, 3, 5), (7, 5, 5));
    fences(f, c, (1, 4, 5), (1, 5, 5));
    fences(f, c, (7, 4, 5), (7, 5, 5));
    for z in 0..=5 {
        for x in 0..=8 {
            down(f, c, x, z);
        }
    }
}
