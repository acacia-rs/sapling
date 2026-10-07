//! The monument's wing rooms (`OceanMonumentWingRoom::postProcess` 14834e440): Java's two
//! designs; BDS's design 0 lacks Java's east doorway pillar.

use super::{BRICKS, DARK, LAMP};
use crate::block::Block;
use crate::structure::canvas::Canvas;
use crate::structure::piece::Frame;

type Cell = (i32, i32, i32);

fn fill(f: &Frame, canvas: &mut Canvas, from: Cell, to: Cell, block: Block) {
    f.generate_box(canvas, from, to, block, block, false);
}

pub fn wing(f: &Frame, design: i32, canvas: &mut Canvas) {
    if design == 0 {
        for i in 0..4 {
            fill(f, canvas, (10 - i, 3 - i, 20 - i), (12 + i, 3 - i, 20), BRICKS);
        }
        for (from, to) in [
            ((7, 0, 6), (15, 0, 16)),
            ((6, 0, 6), (6, 3, 20)),
            ((16, 0, 6), (16, 3, 20)),
            ((7, 1, 7), (7, 1, 20)),
            ((15, 1, 7), (15, 1, 20)),
            ((7, 1, 6), (9, 3, 6)),
            ((13, 1, 6), (15, 3, 6)),
            ((8, 1, 7), (9, 1, 7)),
            ((13, 1, 7), (14, 1, 7)),
            ((9, 0, 5), (13, 0, 5)),
        ] {
            fill(f, canvas, from, to, BRICKS);
        }
        for (from, to) in [((10, 0, 7), (12, 0, 7)), ((8, 0, 10), (8, 0, 12)), ((14, 0, 10), (14, 0, 12))] {
            fill(f, canvas, from, to, DARK);
        }
        for z in (7..=18).rev().step_by(3) {
            f.place(canvas, LAMP, 6, 3, z);
            f.place(canvas, LAMP, 16, 3, z);
        }
        for (x, y, z) in [(10, 0, 10), (12, 0, 10), (10, 0, 12), (12, 0, 12), (8, 3, 6), (14, 3, 6)] {
            f.place(canvas, LAMP, x, y, z);
        }
        for (x, z) in [(4, 4), (18, 4), (4, 18), (18, 18)] {
            f.place(canvas, BRICKS, x, 2, z);
            f.place(canvas, LAMP, x, 1, z);
            f.place(canvas, BRICKS, x, 0, z);
        }
        f.place(canvas, BRICKS, 9, 7, 20);
        f.place(canvas, BRICKS, 13, 7, 20);
        fill(f, canvas, (6, 0, 21), (7, 4, 21), BRICKS);
    } else if design == 1 {
        fill(f, canvas, (9, 3, 18), (13, 3, 20), BRICKS);
        fill(f, canvas, (9, 0, 18), (9, 2, 18), BRICKS);
        fill(f, canvas, (13, 0, 18), (13, 2, 18), BRICKS);
        for x in [9, 13] {
            f.place(canvas, BRICKS, x, 6, 20);
            f.place(canvas, LAMP, x, 5, 20);
            f.place(canvas, BRICKS, x, 4, 20);
        }
        fill(f, canvas, (7, 3, 7), (15, 3, 14), BRICKS);
        for x in [10, 12] {
            fill(f, canvas, (x, 0, 10), (x, 6, 10), BRICKS);
            fill(f, canvas, (x, 0, 12), (x, 6, 12), BRICKS);
            for (y, z) in [(0, 10), (0, 12), (4, 10), (4, 12)] {
                f.place(canvas, LAMP, x, y, z);
            }
        }
        for x in [8, 14] {
            fill(f, canvas, (x, 0, 7), (x, 2, 7), BRICKS);
            fill(f, canvas, (x, 0, 14), (x, 2, 14), BRICKS);
        }
        fill(f, canvas, (8, 3, 8), (8, 3, 13), DARK);
        fill(f, canvas, (14, 3, 8), (14, 3, 13), DARK);
    }
}
