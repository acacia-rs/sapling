//! The monument's shell (`MonumentBuilding::postProcess` 148337c70): wings (148340f10), entrance,
//! roof and the three tiers of walls, each only where it reaches the chunk, then the pillars
//! under the base and the sloped water around it. Unlike Java, BDS clears no water box first.

use super::{BRICKS, DARK, LAMP, PRISMARINE, chunk_intersects, water_box};
use crate::block::Block;
use crate::structure::canvas::Canvas;
use crate::structure::piece::Frame;

type Cell = (i32, i32, i32);

fn fill(f: &Frame, canvas: &mut Canvas, from: Cell, to: Cell, block: Block) {
    f.generate_box(canvas, from, to, block, block, false);
}

pub fn post_process(f: &Frame, canvas: &mut Canvas) {
    wing(f, canvas, false, 0);
    wing(f, canvas, true, 33);
    entrance_arches(f, canvas);
    entrance_wall(f, canvas);
    roof(f, canvas);
    lower_wall(f, canvas);
    super::building_walls::middle_wall(f, canvas);
    super::building_walls::upper_wall(f, canvas);
    for j in 0..7 {
        let mut k = 0;
        while k < 7 {
            if k == 0 && j == 3 {
                k = 6;
            }
            let (l, m) = (j * 9, k * 9);
            for n in 0..4 {
                for o in 0..4 {
                    f.place(canvas, BRICKS, l + n, 0, m + o);
                    f.fill_column_down(canvas, BRICKS, l + n, -1, m + o);
                }
            }
            k += if j == 0 || j == 6 { 1 } else { 6 };
        }
    }
    for j in 0..5 {
        water_box(f, canvas, (-1 - j, j * 2, -1 - j), (-1 - j, 23, 58 + j));
        water_box(f, canvas, (58 + j, j * 2, -1 - j), (58 + j, 23, 58 + j));
        water_box(f, canvas, (-j, j * 2, -1 - j), (57 + j, 23, -1 - j));
        water_box(f, canvas, (-j, j * 2, 58 + j), (57 + j, 23, 58 + j));
    }
}

/// `generateWing`: one of the two side halls, mirrored by `right`.
fn wing(f: &Frame, canvas: &mut Canvas, right: bool, x: i32) {
    if !chunk_intersects(f, canvas, (x, 0), (x + 23, 20)) {
        return;
    }
    fill(f, canvas, (x, 0, 0), (x + 24, 0, 20), PRISMARINE);
    water_box(f, canvas, (x, 1, 0), (x + 24, 10, 20));
    for i in 0..4 {
        fill(f, canvas, (x + i, i + 1, i), (x + i, i + 1, 20), BRICKS);
        fill(f, canvas, (x + i + 7, i + 5, i + 7), (x + i + 7, i + 5, 20), BRICKS);
        fill(f, canvas, (x + 17 - i, i + 5, i + 7), (x + 17 - i, i + 5, 20), BRICKS);
        fill(f, canvas, (x + 24 - i, i + 1, i), (x + 24 - i, i + 1, 20), BRICKS);
        fill(f, canvas, (x + i + 1, i + 1, i), (x + 23 - i, i + 1, i), BRICKS);
        fill(f, canvas, (x + i + 8, i + 5, i + 7), (x + 16 - i, i + 5, i + 7), BRICKS);
    }
    fill(f, canvas, (x + 4, 4, 4), (x + 6, 4, 20), PRISMARINE);
    fill(f, canvas, (x + 7, 4, 4), (x + 17, 4, 6), PRISMARINE);
    fill(f, canvas, (x + 18, 4, 4), (x + 20, 4, 20), PRISMARINE);
    fill(f, canvas, (x + 11, 8, 11), (x + 13, 8, 20), PRISMARINE);
    for z in [12, 15, 18] {
        f.place(canvas, BRICKS, x + 12, 9, z);
    }
    let (j, k) = if right { (x + 19, x + 5) } else { (x + 5, x + 19) };
    for z in (5..=20).rev().step_by(3) {
        f.place(canvas, BRICKS, j, 5, z);
    }
    for z in (7..=19).rev().step_by(3) {
        f.place(canvas, BRICKS, k, 5, z);
    }
    for l in 0..4 {
        let m = if right { x + 24 - (17 - l * 3) } else { x + 17 - l * 3 };
        f.place(canvas, BRICKS, m, 5, 5);
    }
    f.place(canvas, BRICKS, k, 5, 5);
    fill(f, canvas, (x + 11, 1, 12), (x + 13, 7, 12), PRISMARINE);
    fill(f, canvas, (x + 12, 1, 11), (x + 12, 7, 13), PRISMARINE);
}

fn entrance_arches(f: &Frame, canvas: &mut Canvas) {
    if !chunk_intersects(f, canvas, (22, 5), (35, 17)) {
        return;
    }
    water_box(f, canvas, (25, 0, 0), (32, 8, 20));
    for i in 0..4 {
        let z = 5 + i * 4;
        fill(f, canvas, (24, 2, z), (24, 4, z), BRICKS);
        fill(f, canvas, (22, 4, z), (23, 4, z), BRICKS);
        f.place(canvas, BRICKS, 25, 5, z);
        f.place(canvas, BRICKS, 26, 6, z);
        f.place(canvas, LAMP, 26, 5, z);
        fill(f, canvas, (33, 2, z), (33, 4, z), BRICKS);
        fill(f, canvas, (34, 4, z), (35, 4, z), BRICKS);
        f.place(canvas, BRICKS, 32, 5, z);
        f.place(canvas, BRICKS, 31, 6, z);
        f.place(canvas, LAMP, 31, 5, z);
        fill(f, canvas, (27, 6, z), (30, 6, z), PRISMARINE);
    }
}

fn entrance_wall(f: &Frame, canvas: &mut Canvas) {
    if !chunk_intersects(f, canvas, (15, 20), (42, 21)) {
        return;
    }
    fill(f, canvas, (15, 0, 21), (42, 0, 21), PRISMARINE);
    water_box(f, canvas, (26, 1, 21), (31, 3, 21));
    for (x0, y, x1) in [(21, 12, 36), (17, 11, 40), (16, 10, 41)] {
        fill(f, canvas, (x0, y, 21), (x1, y, 21), PRISMARINE);
    }
    fill(f, canvas, (15, 7, 21), (42, 9, 21), PRISMARINE);
    for (x0, y, x1) in [(16, 6, 41), (17, 5, 40), (21, 4, 36), (22, 3, 26), (31, 3, 35), (23, 2, 25), (32, 2, 34)] {
        fill(f, canvas, (x0, y, 21), (x1, y, 21), PRISMARINE);
    }
    fill(f, canvas, (28, 4, 20), (29, 4, 21), BRICKS);
    for (x, y) in [(27, 3), (30, 3), (26, 2), (31, 2), (25, 1), (32, 1)] {
        f.place(canvas, BRICKS, x, y, 21);
    }
    for i in 0..7 {
        f.place(canvas, DARK, 28 - i, 6 + i, 21);
        f.place(canvas, DARK, 29 + i, 6 + i, 21);
    }
    for i in 0..4 {
        f.place(canvas, DARK, 28 - i, 9 + i, 21);
        f.place(canvas, DARK, 29 + i, 9 + i, 21);
    }
    f.place(canvas, DARK, 28, 12, 21);
    f.place(canvas, DARK, 29, 12, 21);
    for i in 0..3 {
        for (x, y) in [(22 - i * 2, 8), (22 - i * 2, 9), (35 + i * 2, 8), (35 + i * 2, 9)] {
            f.place(canvas, DARK, x, y, 21);
        }
    }
    water_box(f, canvas, (15, 13, 21), (42, 15, 21));
    for (x0, y1, x1) in [
        (15, 6, 15),
        (16, 5, 16),
        (17, 4, 20),
        (21, 3, 21),
        (22, 2, 22),
        (23, 1, 24),
        (42, 6, 42),
        (41, 5, 41),
        (37, 4, 40),
        (36, 3, 36),
        (33, 1, 34),
        (35, 2, 35),
    ] {
        water_box(f, canvas, (x0, 1, 21), (x1, y1, 21));
    }
}

fn roof(f: &Frame, canvas: &mut Canvas) {
    if !chunk_intersects(f, canvas, (21, 21), (36, 36)) {
        return;
    }
    fill(f, canvas, (21, 0, 22), (36, 0, 36), PRISMARINE);
    water_box(f, canvas, (21, 1, 22), (36, 23, 36));
    for i in 0..4 {
        let y = 13 + i;
        fill(f, canvas, (21 + i, y, 21 + i), (36 - i, y, 21 + i), BRICKS);
        fill(f, canvas, (21 + i, y, 36 - i), (36 - i, y, 36 - i), BRICKS);
        fill(f, canvas, (21 + i, y, 22 + i), (21 + i, y, 35 - i), BRICKS);
        fill(f, canvas, (36 - i, y, 22 + i), (36 - i, y, 35 - i), BRICKS);
    }
    fill(f, canvas, (25, 16, 25), (32, 16, 32), PRISMARINE);
    for (x, z) in [(25, 25), (32, 25), (25, 32), (32, 32)] {
        fill(f, canvas, (x, 17, z), (x, 19, z), BRICKS);
    }
    for (corner, inner) in [((26, 26), (27, 27)), ((26, 31), (27, 30)), ((31, 31), (30, 30)), ((31, 26), (30, 27))] {
        f.place(canvas, BRICKS, corner.0, 20, corner.1);
        f.place(canvas, BRICKS, inner.0, 21, inner.1);
        f.place(canvas, LAMP, inner.0, 20, inner.1);
    }
    fill(f, canvas, (28, 21, 27), (29, 21, 27), PRISMARINE);
    fill(f, canvas, (27, 21, 28), (27, 21, 29), PRISMARINE);
    fill(f, canvas, (28, 21, 30), (29, 21, 30), PRISMARINE);
    fill(f, canvas, (30, 21, 28), (30, 21, 29), PRISMARINE);
}

fn lower_wall(f: &Frame, canvas: &mut Canvas) {
    if chunk_intersects(f, canvas, (0, 21), (6, 58)) {
        fill(f, canvas, (0, 0, 21), (6, 0, 57), PRISMARINE);
        water_box(f, canvas, (0, 1, 21), (6, 7, 57));
        fill(f, canvas, (4, 4, 21), (6, 4, 53), PRISMARINE);
        for i in 0..4 {
            fill(f, canvas, (i, i + 1, 21), (i, i + 1, 57 - i), BRICKS);
        }
        for z in (23..53).step_by(3).chain([52]) {
            f.place(canvas, BRICKS, 5, 5, z);
        }
        for i in 0..4 {
            fill(f, canvas, (i, i + 1, 21), (i, i + 1, 57 - i), BRICKS);
        }
        fill(f, canvas, (4, 1, 52), (6, 3, 52), PRISMARINE);
        fill(f, canvas, (5, 1, 51), (5, 3, 53), PRISMARINE);
    }
    if chunk_intersects(f, canvas, (51, 21), (58, 58)) {
        fill(f, canvas, (51, 0, 21), (57, 0, 57), PRISMARINE);
        water_box(f, canvas, (51, 1, 21), (57, 7, 57));
        fill(f, canvas, (51, 4, 21), (53, 4, 53), PRISMARINE);
        for i in 0..4 {
            fill(f, canvas, (57 - i, i + 1, 21), (57 - i, i + 1, 57 - i), BRICKS);
        }
        for z in (23..53).step_by(3).chain([52]) {
            f.place(canvas, BRICKS, 52, 5, z);
        }
        fill(f, canvas, (51, 1, 52), (53, 3, 52), PRISMARINE);
        fill(f, canvas, (52, 1, 51), (52, 3, 53), PRISMARINE);
    }
    if chunk_intersects(f, canvas, (0, 51), (57, 57)) {
        fill(f, canvas, (7, 0, 51), (50, 0, 57), PRISMARINE);
        water_box(f, canvas, (7, 1, 51), (50, 10, 57));
        for i in 0..4 {
            fill(f, canvas, (i + 1, i + 1, 57 - i), (56 - i, i + 1, 57 - i), BRICKS);
        }
    }
}
