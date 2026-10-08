//! The monument shell's middle and upper wall tiers (inlined in 148337c70).

use super::{BRICKS, PRISMARINE, chunk_intersects, water_box};
use crate::block::Block;
use crate::structure::canvas::Canvas;
use crate::structure::piece::Frame;

type Cell = (i32, i32, i32);

fn fill(f: &Frame, canvas: &mut Canvas, from: Cell, to: Cell, block: Block) {
    f.generate_box(canvas, from, to, block, block, false);
}

pub fn middle_wall(f: &Frame, canvas: &mut Canvas) {
    if chunk_intersects(f, canvas, (7, 21), (13, 54)) {
        fill(f, canvas, (7, 0, 21), (13, 0, 50), PRISMARINE);
        water_box(f, canvas, (7, 1, 21), (13, 10, 50));
        fill(f, canvas, (11, 8, 21), (13, 8, 53), PRISMARINE);
        for i in 0..4 {
            fill(f, canvas, (i + 7, i + 5, 21), (i + 7, i + 5, 54), BRICKS);
        }
        for z in (21..=45).step_by(3) {
            f.place(canvas, BRICKS, 12, 9, z);
        }
    }
    if chunk_intersects(f, canvas, (44, 21), (50, 54)) {
        fill(f, canvas, (44, 0, 21), (50, 0, 50), PRISMARINE);
        water_box(f, canvas, (44, 1, 21), (50, 10, 50));
        fill(f, canvas, (44, 8, 21), (46, 8, 53), PRISMARINE);
        for i in 0..4 {
            fill(f, canvas, (50 - i, i + 5, 21), (50 - i, i + 5, 54), BRICKS);
        }
        for z in (21..=45).step_by(3) {
            f.place(canvas, BRICKS, 45, 9, z);
        }
    }
    if chunk_intersects(f, canvas, (8, 44), (49, 54)) {
        fill(f, canvas, (14, 0, 44), (43, 0, 50), PRISMARINE);
        water_box(f, canvas, (14, 1, 44), (43, 10, 50));
        for x in (12..=45).step_by(3) {
            f.place(canvas, BRICKS, x, 9, 45);
            f.place(canvas, BRICKS, x, 9, 52);
            if [12, 18, 24, 33, 39, 45].contains(&x) {
                for (y, z) in [(9, 47), (9, 50), (10, 45), (10, 46), (10, 51), (10, 52), (11, 47), (11, 50), (12, 48), (12, 49)] {
                    f.place(canvas, BRICKS, x, y, z);
                }
            }
        }
        for i in 0..3 {
            fill(f, canvas, (8 + i, 5 + i, 54), (49 - i, 5 + i, 54), PRISMARINE);
        }
        fill(f, canvas, (11, 8, 54), (46, 8, 54), BRICKS);
        fill(f, canvas, (14, 8, 44), (43, 8, 53), PRISMARINE);
    }
}

pub fn upper_wall(f: &Frame, canvas: &mut Canvas) {
    if chunk_intersects(f, canvas, (14, 21), (20, 43)) {
        fill(f, canvas, (14, 0, 21), (20, 0, 43), PRISMARINE);
        water_box(f, canvas, (14, 1, 22), (20, 14, 43));
        fill(f, canvas, (18, 12, 22), (20, 12, 39), PRISMARINE);
        fill(f, canvas, (18, 12, 21), (20, 12, 21), BRICKS);
        for i in 0..4 {
            fill(f, canvas, (i + 14, i + 9, 21), (i + 14, i + 9, 43 - i), BRICKS);
        }
        for z in (23..=39).step_by(3) {
            f.place(canvas, BRICKS, 19, 13, z);
        }
    }
    if chunk_intersects(f, canvas, (37, 21), (43, 43)) {
        fill(f, canvas, (37, 0, 21), (43, 0, 43), PRISMARINE);
        water_box(f, canvas, (37, 1, 22), (43, 14, 43));
        fill(f, canvas, (37, 12, 22), (39, 12, 39), PRISMARINE);
        fill(f, canvas, (37, 12, 21), (39, 12, 21), BRICKS);
        for i in 0..4 {
            fill(f, canvas, (43 - i, i + 9, 21), (43 - i, i + 9, 43 - i), BRICKS);
        }
        for z in (23..=39).step_by(3) {
            f.place(canvas, BRICKS, 38, 13, z);
        }
    }
    if chunk_intersects(f, canvas, (15, 37), (42, 43)) {
        fill(f, canvas, (21, 0, 37), (36, 0, 43), PRISMARINE);
        water_box(f, canvas, (21, 1, 37), (36, 14, 43));
        fill(f, canvas, (21, 12, 37), (36, 12, 39), PRISMARINE);
        for i in 0..4 {
            fill(f, canvas, (15 + i, i + 9, 43 - i), (42 - i, i + 9, 43 - i), BRICKS);
        }
        for x in (21..=36).step_by(3) {
            f.place(canvas, BRICKS, x, 13, 38);
        }
    }
}
