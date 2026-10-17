//! The two 13x13 castle halls (14832a9d0 entrance, 14832c1b0 stalk room): one shell, then the
//! entrance's lava well or the stalk room's stairs and nether wart.

use super::{air, bricks, down, fences, fill, stairs};
use crate::block::Block;
use crate::structure::canvas::Canvas;
use crate::structure::piece::Frame;

/// Walls, roof, battlements and the window fences both halls share.
fn shell(f: &Frame, c: &mut Canvas, gate: bool) {
    bricks(f, c, (0, 3, 0), (12, 4, 12));
    air(f, c, (0, 5, 0), (12, 13, 12));
    bricks(f, c, (0, 5, 0), (1, 12, 12));
    bricks(f, c, (11, 5, 0), (12, 12, 12));
    bricks(f, c, (2, 5, 11), (4, 12, 12));
    bricks(f, c, (8, 5, 11), (10, 12, 12));
    bricks(f, c, (5, 9, 11), (7, 12, 12));
    bricks(f, c, (2, 5, 0), (4, 12, 1));
    bricks(f, c, (8, 5, 0), (10, 12, 1));
    bricks(f, c, (5, 9, 0), (7, 12, 1));
    bricks(f, c, (2, 11, 2), (10, 12, 10));
    if gate {
        fences(f, c, (5, 8, 0), (7, 8, 0));
    }
    // BDS also sets the i + 1 fences at i = 11, which the corners below set anyway.
    for i in (1..=11).step_by(2) {
        fences(f, c, (i, 10, 0), (i, 11, 0));
        fences(f, c, (i, 10, 12), (i, 11, 12));
        fences(f, c, (0, 10, i), (0, 11, i));
        fences(f, c, (12, 10, i), (12, 11, i));
        for (x, z) in [(i, 0), (i, 12), (0, i), (12, i)] {
            f.place(c, Block::NetherBrick, x, 13, z);
        }
        for (x, z) in [(i + 1, 0), (i + 1, 12), (0, i + 1), (12, i + 1)] {
            f.place(c, Block::NetherBrickFence0, x, 13, z);
        }
    }
    for (x, z) in [(0, 0), (0, 12), (12, 12), (12, 0)] {
        f.place(c, Block::NetherBrickFence0, x, 13, z);
    }
    for z in (3..=9).step_by(2) {
        fences(f, c, (1, 7, z), (1, 8, z));
        fences(f, c, (11, 7, z), (11, 8, z));
    }
}

/// The cross-shaped base under both halls, filled down.
fn base(f: &Frame, c: &mut Canvas) {
    bricks(f, c, (4, 2, 0), (8, 2, 12));
    bricks(f, c, (0, 2, 4), (12, 2, 8));
    bricks(f, c, (4, 0, 0), (8, 1, 3));
    bricks(f, c, (4, 0, 9), (8, 1, 12));
    bricks(f, c, (0, 0, 4), (3, 1, 8));
    bricks(f, c, (9, 0, 4), (12, 1, 8));
    for x in 4..=8 {
        for z in 0..=2 {
            down(f, c, x, z);
            down(f, c, x, 12 - z);
        }
    }
    for x in 0..=2 {
        for z in 4..=8 {
            down(f, c, x, z);
            down(f, c, 12 - x, z);
        }
    }
}

pub fn entrance(f: &Frame, c: &mut Canvas) {
    shell(f, c, true);
    base(f, c);
    bricks(f, c, (5, 5, 5), (7, 5, 7));
    air(f, c, (6, 1, 6), (6, 4, 6));
    f.place(c, Block::NetherBrick, 6, 0, 6);
    f.place(c, Block::Lava, 6, 5, 6);
}

pub fn stalk_room(f: &Frame, c: &mut Canvas) {
    shell(f, c, false);
    let flight = stairs(f, 3);
    for k in 0..=6 {
        let z = k + 4;
        for x in 5..=7 {
            f.place(c, flight, x, 5 + k, z);
        }
        if (1..=4).contains(&k) {
            bricks(f, c, (5, 5, z), (7, k + 4, z));
        } else if k >= 5 {
            bricks(f, c, (5, 8, z), (7, k + 4, z));
        }
        if k >= 1 {
            air(f, c, (5, 6 + k, z), (7, 9 + k, z));
        }
    }
    for x in 5..=7 {
        f.place(c, flight, x, 12, 11);
    }
    fences(f, c, (5, 6, 7), (5, 7, 7));
    fences(f, c, (7, 6, 7), (7, 7, 7));
    air(f, c, (5, 13, 12), (7, 13, 12));
    bricks(f, c, (2, 5, 2), (3, 5, 3));
    bricks(f, c, (2, 5, 9), (3, 5, 10));
    bricks(f, c, (2, 5, 4), (2, 5, 8));
    bricks(f, c, (9, 5, 2), (10, 5, 3));
    bricks(f, c, (9, 5, 9), (10, 5, 10));
    bricks(f, c, (10, 5, 4), (10, 5, 8));
    let (west, east) = (stairs(f, 1), stairs(f, 0));
    for z in [2, 3, 9, 10] {
        f.place(c, west, 4, 5, z);
    }
    for z in [2, 3, 9, 10] {
        f.place(c, east, 8, 5, z);
    }
    fill(f, c, (3, 4, 4), (4, 4, 8), Block::SoulSand);
    fill(f, c, (8, 4, 4), (9, 4, 8), Block::SoulSand);
    fill(f, c, (3, 5, 4), (4, 5, 8), Block::NetherWart);
    fill(f, c, (8, 5, 4), (9, 5, 8), Block::NetherWart);
    base(f, c);
}
