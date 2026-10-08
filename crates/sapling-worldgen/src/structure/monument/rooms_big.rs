//! The monument's larger rooms: double Z (148348cc0), double XY (14834a2b0), double YZ (14834bb70),
//! the core (14834d1c0), the wings (14834e440) and the penthouse (14834fb70), as Java's.

use super::graph::{DOWN, EAST, NORTH, SOUTH, WEST};
use super::{BRICKS, DARK, LAMP, PRISMARINE, Piece, box_on_water, default_floor, water_box};
use crate::block::Block;
use crate::structure::canvas::Canvas;
use crate::structure::piece::Frame;

type Cell = (i32, i32, i32);

fn fill(f: &Frame, canvas: &mut Canvas, from: Cell, to: Cell, block: Block) {
    f.generate_box(canvas, from, to, block, block, false);
}

fn water_where(f: &Frame, canvas: &mut Canvas, boxes: &[(bool, Cell, Cell)]) {
    for &(open, from, to) in boxes {
        if open {
            water_box(f, canvas, from, to);
        }
    }
}

/// Two-high walls: prismarine at y 2 and 6, bricks elsewhere.
fn layered(y: i32) -> Block {
    if y == 2 || y == 6 { PRISMARINE } else { BRICKS }
}

pub fn double_z(f: &Frame, piece: &Piece, canvas: &mut Canvas) {
    let (south, north) = (piece.rooms[0], piece.rooms[1]);
    if piece.index / 25 > 0 {
        default_floor(f, canvas, 0, 8, north.open[DOWN]);
        default_floor(f, canvas, 0, 0, south.open[DOWN]);
    }
    if !south.linked_up {
        box_on_water(f, canvas, (1, 4, 1), (6, 4, 7), PRISMARINE);
    }
    if !north.linked_up {
        box_on_water(f, canvas, (1, 4, 8), (6, 4, 14), PRISMARINE);
    }
    for (y, block) in [(3, BRICKS), (2, PRISMARINE), (1, BRICKS)] {
        fill(f, canvas, (0, y, 0), (0, y, 15), block);
        fill(f, canvas, (7, y, 0), (7, y, 15), block);
        fill(f, canvas, (1, y, 0), (7, y, 0), block);
        fill(f, canvas, (1, y, 15), (6, y, 15), block);
    }
    for (from, to) in [
        ((1, 1, 1), (1, 1, 2)),
        ((6, 1, 1), (6, 1, 2)),
        ((1, 3, 1), (1, 3, 2)),
        ((6, 3, 1), (6, 3, 2)),
        ((1, 1, 13), (1, 1, 14)),
        ((6, 1, 13), (6, 1, 14)),
        ((1, 3, 13), (1, 3, 14)),
        ((6, 3, 13), (6, 3, 14)),
        ((2, 1, 6), (2, 3, 6)),
        ((5, 1, 6), (5, 3, 6)),
        ((2, 1, 9), (2, 3, 9)),
        ((5, 1, 9), (5, 3, 9)),
        ((3, 2, 6), (4, 2, 6)),
        ((3, 2, 9), (4, 2, 9)),
        ((2, 2, 7), (2, 2, 8)),
        ((5, 2, 7), (5, 2, 8)),
    ] {
        fill(f, canvas, from, to, BRICKS);
    }
    for (x, z) in [(2, 5), (5, 5), (2, 10), (5, 10)] {
        f.place(canvas, LAMP, x, 2, z);
    }
    for (x, z) in [(2, 5), (5, 5), (2, 10), (5, 10)] {
        f.place(canvas, BRICKS, x, 3, z);
    }
    water_where(
        f,
        canvas,
        &[
            (south.open[SOUTH], (3, 1, 0), (4, 2, 0)),
            (south.open[EAST], (7, 1, 3), (7, 2, 4)),
            (south.open[WEST], (0, 1, 3), (0, 2, 4)),
            (north.open[NORTH], (3, 1, 15), (4, 2, 15)),
            (north.open[WEST], (0, 1, 11), (0, 2, 12)),
            (north.open[EAST], (7, 1, 11), (7, 2, 12)),
        ],
    );
}

pub fn double_xy(f: &Frame, piece: &Piece, canvas: &mut Canvas) {
    let [low, east, up, east_up] = [0, 1, 2, 3].map(|i| piece.rooms[i]);
    if piece.index / 25 > 0 {
        default_floor(f, canvas, 8, 0, east.open[DOWN]);
        default_floor(f, canvas, 0, 0, low.open[DOWN]);
    }
    if !up.linked_up {
        box_on_water(f, canvas, (1, 8, 1), (7, 8, 6), PRISMARINE);
    }
    if !east_up.linked_up {
        box_on_water(f, canvas, (8, 8, 1), (14, 8, 6), PRISMARINE);
    }
    for y in 1..=7 {
        let block = layered(y);
        fill(f, canvas, (0, y, 0), (0, y, 7), block);
        fill(f, canvas, (15, y, 0), (15, y, 7), block);
        fill(f, canvas, (1, y, 0), (15, y, 0), block);
        fill(f, canvas, (1, y, 7), (14, y, 7), block);
    }
    for (from, to) in [
        ((2, 1, 3), (2, 7, 4)),
        ((3, 1, 2), (4, 7, 2)),
        ((3, 1, 5), (4, 7, 5)),
        ((13, 1, 3), (13, 7, 4)),
        ((11, 1, 2), (12, 7, 2)),
        ((11, 1, 5), (12, 7, 5)),
        ((5, 1, 3), (5, 3, 4)),
        ((10, 1, 3), (10, 3, 4)),
        ((5, 7, 2), (10, 7, 5)),
        ((5, 5, 2), (5, 7, 2)),
        ((10, 5, 2), (10, 7, 2)),
        ((5, 5, 5), (5, 7, 5)),
        ((10, 5, 5), (10, 7, 5)),
    ] {
        fill(f, canvas, from, to, BRICKS);
    }
    for (x, z) in [(6, 2), (9, 2), (6, 5), (9, 5)] {
        f.place(canvas, BRICKS, x, 6, z);
    }
    fill(f, canvas, (5, 4, 3), (6, 4, 4), BRICKS);
    fill(f, canvas, (9, 4, 3), (10, 4, 4), BRICKS);
    for (x, z) in [(5, 2), (5, 5), (10, 2), (10, 5)] {
        f.place(canvas, LAMP, x, 4, z);
    }
    let mut boxes = Vec::new();
    for (y, west, east) in [(1, low, east), (5, up, east_up)] {
        boxes.extend([
            (west.open[SOUTH], (3, y, 0), (4, y + 1, 0)),
            (west.open[NORTH], (3, y, 7), (4, y + 1, 7)),
            (west.open[WEST], (0, y, 3), (0, y + 1, 4)),
            (east.open[SOUTH], (11, y, 0), (12, y + 1, 0)),
            (east.open[NORTH], (11, y, 7), (12, y + 1, 7)),
            (east.open[EAST], (15, y, 3), (15, y + 1, 4)),
        ]);
    }
    water_where(f, canvas, &boxes);
}

pub fn double_yz(f: &Frame, piece: &Piece, canvas: &mut Canvas) {
    let [low, north, up, north_up] = [0, 1, 2, 3].map(|i| piece.rooms[i]);
    if piece.index / 25 > 0 {
        default_floor(f, canvas, 0, 8, north.open[DOWN]);
        default_floor(f, canvas, 0, 0, low.open[DOWN]);
    }
    if !up.linked_up {
        box_on_water(f, canvas, (1, 8, 1), (6, 8, 7), PRISMARINE);
    }
    if !north_up.linked_up {
        box_on_water(f, canvas, (1, 8, 8), (6, 8, 14), PRISMARINE);
    }
    for y in 1..=7 {
        let block = layered(y);
        fill(f, canvas, (0, y, 0), (0, y, 15), block);
        fill(f, canvas, (7, y, 0), (7, y, 15), block);
        fill(f, canvas, (1, y, 0), (6, y, 0), block);
        fill(f, canvas, (1, y, 15), (6, y, 15), block);
    }
    for y in 1..=7 {
        fill(f, canvas, (3, y, 7), (4, y, 8), if y == 2 || y == 6 { LAMP } else { DARK });
    }
    water_where(
        f,
        canvas,
        &[
            (low.open[SOUTH], (3, 1, 0), (4, 2, 0)),
            (low.open[EAST], (7, 1, 3), (7, 2, 4)),
            (low.open[WEST], (0, 1, 3), (0, 2, 4)),
            (north.open[NORTH], (3, 1, 15), (4, 2, 15)),
            (north.open[WEST], (0, 1, 11), (0, 2, 12)),
            (north.open[EAST], (7, 1, 11), (7, 2, 12)),
            (up.open[SOUTH], (3, 5, 0), (4, 6, 0)),
        ],
    );
    // Upper side openings get a frame of bricks round the water.
    let frames: [(bool, Cell, Cell, &[(Cell, Cell)]); 5] = [
        (up.open[EAST], (7, 5, 3), (7, 6, 4), &[((5, 4, 2), (6, 4, 5)), ((6, 1, 2), (6, 3, 2)), ((6, 1, 5), (6, 3, 5))]),
        (up.open[WEST], (0, 5, 3), (0, 6, 4), &[((1, 4, 2), (2, 4, 5)), ((1, 1, 2), (1, 3, 2)), ((1, 1, 5), (1, 3, 5))]),
        (north_up.open[NORTH], (3, 5, 15), (4, 6, 15), &[]),
        (north_up.open[WEST], (0, 5, 11), (0, 6, 12), &[((1, 4, 10), (2, 4, 13)), ((1, 1, 10), (1, 3, 10)), ((1, 1, 13), (1, 3, 13))]),
        (north_up.open[EAST], (7, 5, 11), (7, 6, 12), &[((5, 4, 10), (6, 4, 13)), ((6, 1, 10), (6, 3, 10)), ((6, 1, 13), (6, 3, 13))]),
    ];
    for (open, from, to, bricks) in frames {
        if open {
            water_box(f, canvas, from, to);
            for &(a, b) in bricks {
                fill(f, canvas, a, b, BRICKS);
            }
        }
    }
}

pub fn core(f: &Frame, canvas: &mut Canvas) {
    box_on_water(f, canvas, (1, 8, 0), (14, 8, 14), PRISMARINE);
    fill(f, canvas, (0, 7, 0), (0, 7, 15), BRICKS);
    fill(f, canvas, (15, 7, 0), (15, 7, 15), BRICKS);
    fill(f, canvas, (1, 7, 0), (15, 7, 0), BRICKS);
    fill(f, canvas, (1, 7, 15), (14, 7, 15), BRICKS);
    for y in 1..=6 {
        let block = layered(y);
        for x in [0, 15] {
            fill(f, canvas, (x, y, 0), (x, y, 1), block);
            fill(f, canvas, (x, y, 6), (x, y, 9), block);
            fill(f, canvas, (x, y, 14), (x, y, 15), block);
        }
        fill(f, canvas, (1, y, 0), (1, y, 0), block);
        fill(f, canvas, (6, y, 0), (9, y, 0), block);
        fill(f, canvas, (14, y, 0), (14, y, 0), block);
        fill(f, canvas, (1, y, 15), (14, y, 15), block);
    }
    fill(f, canvas, (6, 3, 6), (9, 6, 9), DARK);
    fill(f, canvas, (7, 4, 7), (8, 5, 8), Block::GoldBlock);
    for y in [3, 6] {
        for x in [6, 9] {
            f.place(canvas, LAMP, x, y, 6);
            f.place(canvas, LAMP, x, y, 9);
        }
    }
    for (from, to) in CORE_BRICKS {
        fill(f, canvas, from, to, BRICKS);
    }
}

const CORE_BRICKS: [(Cell, Cell); 28] = [
    ((5, 1, 6), (5, 2, 6)),
    ((5, 1, 9), (5, 2, 9)),
    ((10, 1, 6), (10, 2, 6)),
    ((10, 1, 9), (10, 2, 9)),
    ((6, 1, 5), (6, 2, 5)),
    ((9, 1, 5), (9, 2, 5)),
    ((6, 1, 10), (6, 2, 10)),
    ((9, 1, 10), (9, 2, 10)),
    ((5, 2, 5), (5, 6, 5)),
    ((5, 2, 10), (5, 6, 10)),
    ((10, 2, 5), (10, 6, 5)),
    ((10, 2, 10), (10, 6, 10)),
    ((5, 7, 1), (5, 7, 6)),
    ((10, 7, 1), (10, 7, 6)),
    ((5, 7, 9), (5, 7, 14)),
    ((10, 7, 9), (10, 7, 14)),
    ((1, 7, 5), (6, 7, 5)),
    ((1, 7, 10), (6, 7, 10)),
    ((9, 7, 5), (14, 7, 5)),
    ((9, 7, 10), (14, 7, 10)),
    ((2, 1, 2), (2, 1, 3)),
    ((3, 1, 2), (3, 1, 2)),
    ((13, 1, 2), (13, 1, 3)),
    ((12, 1, 2), (12, 1, 2)),
    ((2, 1, 12), (2, 1, 13)),
    ((3, 1, 13), (3, 1, 13)),
    ((13, 1, 12), (13, 1, 13)),
    ((12, 1, 13), (12, 1, 13)),
];

pub fn penthouse(f: &Frame, canvas: &mut Canvas) {
    fill(f, canvas, (2, -1, 2), (11, -1, 11), BRICKS);
    for (from, to) in [((0, -1, 0), (1, -1, 11)), ((12, -1, 0), (13, -1, 11)), ((2, -1, 0), (11, -1, 1)), ((2, -1, 12), (11, -1, 13))] {
        fill(f, canvas, from, to, PRISMARINE);
    }
    for (from, to) in [((0, 0, 0), (0, 0, 13)), ((13, 0, 0), (13, 0, 13)), ((1, 0, 0), (12, 0, 0)), ((1, 0, 13), (12, 0, 13))] {
        fill(f, canvas, from, to, BRICKS);
    }
    for i in (2..=11).step_by(3) {
        f.place(canvas, LAMP, 0, 0, i);
        f.place(canvas, LAMP, 13, 0, i);
        f.place(canvas, LAMP, i, 0, 0);
    }
    for (from, to) in [((2, 0, 3), (4, 0, 9)), ((9, 0, 3), (11, 0, 9)), ((4, 0, 9), (9, 0, 11))] {
        fill(f, canvas, from, to, BRICKS);
    }
    for (x, z) in [(5, 8), (8, 8), (10, 10), (3, 10)] {
        f.place(canvas, BRICKS, x, 0, z);
    }
    for (from, to) in [((3, 0, 3), (3, 0, 7)), ((10, 0, 3), (10, 0, 7)), ((6, 0, 10), (7, 0, 10))] {
        fill(f, canvas, from, to, DARK);
    }
    for x in [3, 10] {
        for z in (2..=8).step_by(3) {
            fill(f, canvas, (x, 0, z), (x, 2, z), BRICKS);
        }
    }
    fill(f, canvas, (5, 0, 10), (5, 2, 10), BRICKS);
    fill(f, canvas, (8, 0, 10), (8, 2, 10), BRICKS);
    fill(f, canvas, (6, -1, 7), (7, -1, 8), DARK);
    water_box(f, canvas, (6, -1, 3), (7, -1, 4));
}
