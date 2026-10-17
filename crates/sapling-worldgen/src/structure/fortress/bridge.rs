//! The bridge pieces' post-processes: straights, crossings, the end filler and the bridge rooms
//! (148327f60, 148326e80, 1483289b0, 148329050, 1483297e0, 148329f80).

use super::{air, bricks, down, fences};
use crate::block::Block;
use crate::random::Mt19937;
use crate::structure::canvas::Canvas;
use crate::structure::piece::Frame;

pub fn straight(f: &Frame, c: &mut Canvas) {
    bricks(f, c, (0, 3, 0), (4, 4, 18));
    air(f, c, (1, 5, 0), (3, 7, 18));
    bricks(f, c, (0, 5, 0), (0, 5, 18));
    bricks(f, c, (4, 5, 0), (4, 5, 18));
    bricks(f, c, (0, 2, 0), (4, 2, 5));
    bricks(f, c, (0, 2, 13), (4, 2, 18));
    bricks(f, c, (0, 0, 0), (4, 1, 3));
    bricks(f, c, (0, 0, 15), (4, 1, 18));
    for x in 0..=4 {
        for z in 0..=2 {
            down(f, c, x, z);
            down(f, c, x, 18 - z);
        }
    }
    for x in [0, 4] {
        fences(f, c, (x, 1, 1), (x, 4, 1));
        fences(f, c, (x, 3, 4), (x, 4, 4));
        fences(f, c, (x, 3, 14), (x, 4, 14));
        fences(f, c, (x, 1, 17), (x, 4, 17));
    }
}

pub fn crossing(f: &Frame, c: &mut Canvas) {
    bricks(f, c, (7, 3, 0), (11, 4, 18));
    bricks(f, c, (0, 3, 7), (18, 4, 11));
    air(f, c, (8, 5, 0), (10, 7, 18));
    air(f, c, (0, 5, 8), (18, 7, 10));
    bricks(f, c, (7, 5, 0), (7, 5, 7));
    bricks(f, c, (7, 5, 11), (7, 5, 18));
    bricks(f, c, (11, 5, 0), (11, 5, 7));
    bricks(f, c, (11, 5, 11), (11, 5, 18));
    bricks(f, c, (0, 5, 7), (7, 5, 7));
    bricks(f, c, (11, 5, 7), (18, 5, 7));
    bricks(f, c, (0, 5, 11), (7, 5, 11));
    bricks(f, c, (11, 5, 11), (18, 5, 11));
    bricks(f, c, (7, 2, 0), (11, 2, 5));
    bricks(f, c, (7, 2, 13), (11, 2, 18));
    bricks(f, c, (7, 0, 0), (11, 1, 3));
    bricks(f, c, (7, 0, 15), (11, 1, 18));
    for x in 7..=11 {
        for z in 0..=2 {
            down(f, c, x, z);
            down(f, c, x, 18 - z);
        }
    }
    bricks(f, c, (0, 2, 7), (5, 2, 11));
    bricks(f, c, (13, 2, 7), (18, 2, 11));
    bricks(f, c, (0, 0, 7), (3, 1, 11));
    bricks(f, c, (15, 0, 7), (18, 1, 11));
    for x in 0..=2 {
        for z in 7..=11 {
            down(f, c, x, z);
            down(f, c, 18 - x, z);
        }
    }
}

/// Ragged bridge ends from the filler's own MT: every length is `genrand % n`.
pub fn end_filler(f: &Frame, c: &mut Canvas, seed: u32) {
    let mut random = Mt19937::new(seed);
    for x in 0..=4 {
        for y in 3..=4 {
            let z = random.next_int(8);
            bricks(f, c, (x, y, 0), (x, y, z));
        }
    }
    let z = random.next_int(8);
    bricks(f, c, (0, 5, 0), (0, 5, z));
    let z = random.next_int(8);
    bricks(f, c, (4, 5, 0), (4, 5, z));
    for x in 0..=4 {
        let z = random.next_int(5);
        bricks(f, c, (x, 2, 0), (x, 2, z));
    }
    for x in 0..=4 {
        for y in 0..=1 {
            let z = random.next_int(3);
            bricks(f, c, (x, y, 0), (x, y, z));
        }
    }
}

/// The 7x7 floor of a bridge room, filled down.
fn room_floor(f: &Frame, c: &mut Canvas) {
    for x in 0..=6 {
        for z in 0..=6 {
            down(f, c, x, z);
        }
    }
}

pub fn room_crossing(f: &Frame, c: &mut Canvas) {
    bricks(f, c, (0, 0, 0), (6, 1, 6));
    air(f, c, (0, 2, 0), (6, 7, 6));
    bricks(f, c, (0, 2, 0), (1, 6, 0));
    bricks(f, c, (0, 2, 6), (1, 6, 6));
    bricks(f, c, (5, 2, 0), (6, 6, 0));
    bricks(f, c, (5, 2, 6), (6, 6, 6));
    bricks(f, c, (0, 2, 0), (0, 6, 1));
    bricks(f, c, (0, 2, 5), (0, 6, 6));
    bricks(f, c, (6, 2, 0), (6, 6, 1));
    bricks(f, c, (6, 2, 5), (6, 6, 6));
    bricks(f, c, (2, 6, 0), (4, 6, 0));
    fences(f, c, (2, 5, 0), (4, 5, 0));
    bricks(f, c, (2, 6, 6), (4, 6, 6));
    fences(f, c, (2, 5, 6), (4, 5, 6));
    bricks(f, c, (0, 6, 2), (0, 6, 4));
    fences(f, c, (0, 5, 2), (0, 5, 4));
    bricks(f, c, (6, 6, 2), (6, 6, 4));
    fences(f, c, (6, 5, 2), (6, 5, 4));
    room_floor(f, c);
}

pub fn stairs_room(f: &Frame, c: &mut Canvas) {
    bricks(f, c, (0, 0, 0), (6, 1, 6));
    air(f, c, (0, 2, 0), (6, 10, 6));
    bricks(f, c, (0, 2, 0), (1, 8, 0));
    bricks(f, c, (5, 2, 0), (6, 8, 0));
    bricks(f, c, (0, 2, 1), (0, 8, 6));
    bricks(f, c, (6, 2, 1), (6, 8, 6));
    bricks(f, c, (1, 2, 6), (5, 8, 6));
    fences(f, c, (0, 3, 2), (0, 5, 4));
    fences(f, c, (6, 3, 2), (6, 5, 2));
    fences(f, c, (6, 3, 4), (6, 5, 4));
    f.place(c, Block::NetherBrick, 5, 2, 5);
    bricks(f, c, (4, 2, 5), (4, 3, 5));
    bricks(f, c, (3, 2, 5), (3, 4, 5));
    bricks(f, c, (2, 2, 5), (2, 5, 5));
    bricks(f, c, (1, 2, 5), (1, 6, 5));
    bricks(f, c, (1, 7, 1), (5, 7, 4));
    air(f, c, (6, 8, 2), (6, 8, 4));
    bricks(f, c, (2, 6, 0), (4, 8, 0));
    fences(f, c, (2, 5, 0), (4, 5, 0));
    room_floor(f, c);
}

pub fn monster_throne(f: &Frame, c: &mut Canvas) {
    air(f, c, (0, 2, 0), (6, 7, 7));
    bricks(f, c, (1, 0, 0), (5, 1, 7));
    bricks(f, c, (1, 2, 1), (5, 2, 7));
    bricks(f, c, (1, 3, 2), (5, 3, 7));
    bricks(f, c, (1, 4, 3), (5, 4, 7));
    bricks(f, c, (1, 2, 0), (1, 4, 2));
    bricks(f, c, (5, 2, 0), (5, 4, 2));
    bricks(f, c, (1, 5, 2), (1, 5, 3));
    bricks(f, c, (5, 5, 2), (5, 5, 3));
    bricks(f, c, (0, 5, 3), (0, 5, 8));
    bricks(f, c, (6, 5, 3), (6, 5, 8));
    bricks(f, c, (1, 5, 8), (5, 5, 8));
    f.place(c, Block::NetherBrickFence0, 1, 6, 3);
    f.place(c, Block::NetherBrickFence0, 5, 6, 3);
    fences(f, c, (0, 6, 3), (0, 6, 8));
    fences(f, c, (6, 6, 3), (6, 6, 8));
    fences(f, c, (1, 6, 8), (5, 7, 8));
    fences(f, c, (2, 8, 8), (4, 8, 8));
    // No "already placed" flag: every chunk holding the cell sets it.
    let (x, y, z) = f.world(3, 5, 5);
    if c.bb.contains(x, y, z) {
        c.set(x, y, z, Block::MobSpawner);
    }
    room_floor(f, c);
}
