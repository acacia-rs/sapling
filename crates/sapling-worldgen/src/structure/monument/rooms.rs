//! The monument's one-cell and two-cell rooms: entry (148342020), simple (148342c50), simple top
//! (148345850), double Y (148346980) and double X (1483478c0), as Java's.

use super::graph::{DOWN, EAST, NORTH, SOUTH, UP, WEST};
use super::{BRICKS, DARK, LAMP, PRISMARINE, Piece, box_on_water, default_floor, water_box};
use crate::block::Block;
use crate::random::Mt19937;
use crate::structure::canvas::Canvas;
use crate::structure::piece::Frame;

type Cell = (i32, i32, i32);

fn fill(f: &Frame, canvas: &mut Canvas, from: Cell, to: Cell, block: Block) {
    f.generate_box(canvas, from, to, block, block, false);
}

pub fn entry(f: &Frame, piece: &Piece, canvas: &mut Canvas) {
    for (from, to) in [((0, 3, 0), (2, 3, 7)), ((5, 3, 0), (7, 3, 7)), ((0, 2, 0), (1, 2, 7)), ((6, 2, 0), (7, 2, 7))] {
        fill(f, canvas, from, to, BRICKS);
    }
    for (from, to) in [((0, 1, 0), (0, 1, 7)), ((7, 1, 0), (7, 1, 7)), ((0, 1, 7), (7, 3, 7)), ((1, 1, 0), (2, 3, 0)), ((5, 1, 0), (6, 3, 0))] {
        fill(f, canvas, from, to, BRICKS);
    }
    let open = piece.rooms[0].open;
    if open[NORTH] {
        water_box(f, canvas, (3, 1, 7), (4, 2, 7));
    }
    if open[WEST] {
        water_box(f, canvas, (0, 1, 3), (1, 2, 4));
    }
    if open[EAST] {
        water_box(f, canvas, (6, 1, 3), (7, 2, 4));
    }
}

/// Floor and ceiling every grid room shares: a floor above the ground tier, and the ceiling
/// cells still under water where no room lies above.
fn floor_and_ceiling(f: &Frame, piece: &Piece, canvas: &mut Canvas) {
    if piece.index / 25 > 0 {
        default_floor(f, canvas, 0, 0, piece.rooms[0].open[DOWN]);
    }
    if !piece.rooms[0].linked_up {
        box_on_water(f, canvas, (1, 4, 1), (6, 4, 6), PRISMARINE);
    }
}

pub fn simple(f: &Frame, piece: &Piece, design: i32, canvas: &mut Canvas, random: &mut Mt19937) {
    floor_and_ceiling(f, piece, canvas);
    let open = piece.rooms[0].open;
    let openings = [NORTH, SOUTH, WEST, EAST].iter().filter(|&&face| open[face]).count();
    let pillar = design != 0 && random.next_boolean() && !open[DOWN] && !open[UP] && openings > 1;
    match design {
        0 => {
            for (x0, x1, edge_x, z0, z1, edge_z) in [(0, 2, 0, 0, 2, 0), (5, 7, 7, 0, 2, 0), (0, 2, 0, 5, 7, 7), (5, 7, 7, 5, 7, 7)] {
                fill(f, canvas, (x0, 1, z0), (x1, 1, z1), BRICKS);
                fill(f, canvas, (x0, 3, z0), (x1, 3, z1), BRICKS);
                fill(f, canvas, (edge_x, 2, z0), (edge_x, 2, z1), PRISMARINE);
                let (rx0, rx1) = if edge_x == 0 { (1, 2) } else { (5, 6) };
                fill(f, canvas, (rx0, 2, edge_z), (rx1, 2, edge_z), PRISMARINE);
                f.place(canvas, LAMP, if edge_x == 0 { 1 } else { 6 }, 2, if edge_z == 0 { 1 } else { 6 });
            }
            // Each side: a lintel over an opening, else a wall with a prismarine band.
            let sides: [(usize, Cell, Cell, Cell, Cell, Cell, Cell); 4] = [
                (SOUTH, (3, 3, 0), (4, 3, 0), (3, 3, 0), (4, 3, 1), (3, 2, 0), (4, 2, 0)),
                (NORTH, (3, 3, 7), (4, 3, 7), (3, 3, 6), (4, 3, 7), (3, 2, 7), (4, 2, 7)),
                (WEST, (0, 3, 3), (0, 3, 4), (0, 3, 3), (1, 3, 4), (0, 2, 3), (0, 2, 4)),
                (EAST, (7, 3, 3), (7, 3, 4), (6, 3, 3), (7, 3, 4), (7, 2, 3), (7, 2, 4)),
            ];
            for (face, lintel_a, lintel_b, top_a, top_b, band_a, band_b) in sides {
                if open[face] {
                    fill(f, canvas, lintel_a, lintel_b, BRICKS);
                } else {
                    fill(f, canvas, top_a, top_b, BRICKS);
                    fill(f, canvas, band_a, band_b, PRISMARINE);
                    fill(f, canvas, (top_a.0, 1, top_a.2), (top_b.0, 1, top_b.2), BRICKS);
                }
            }
        }
        1 => {
            for (x, z) in [(2, 2), (2, 5), (5, 5), (5, 2)] {
                fill(f, canvas, (x, 1, z), (x, 3, z), BRICKS);
            }
            for (x, z) in [(2, 2), (2, 5), (5, 5), (5, 2)] {
                f.place(canvas, LAMP, x, 2, z);
            }
            for (from, to) in [
                ((0, 0), (1, 0)),
                ((0, 1), (0, 1)),
                ((0, 7), (1, 7)),
                ((0, 6), (0, 6)),
                ((6, 7), (7, 7)),
                ((7, 6), (7, 6)),
                ((6, 0), (7, 0)),
                ((7, 1), (7, 1)),
            ] {
                fill(f, canvas, (from.0, 1, from.1), (to.0, 3, to.1), BRICKS);
            }
            for (x, z) in [(1, 0), (0, 1), (1, 7), (0, 6), (6, 7), (7, 6), (6, 0), (7, 1)] {
                f.place(canvas, PRISMARINE, x, 2, z);
            }
            for (face, from, to) in [(SOUTH, (1, 0), (6, 0)), (NORTH, (1, 7), (6, 7)), (WEST, (0, 1), (0, 6)), (EAST, (7, 1), (7, 6))] {
                if !open[face] {
                    fill(f, canvas, (from.0, 3, from.1), (to.0, 3, to.1), BRICKS);
                    fill(f, canvas, (from.0, 2, from.1), (to.0, 2, to.1), PRISMARINE);
                    fill(f, canvas, (from.0, 1, from.1), (to.0, 1, to.1), BRICKS);
                }
            }
        }
        2 => {
            for (y, block) in [(1, BRICKS), (2, DARK), (3, BRICKS)] {
                fill(f, canvas, (0, y, 0), (0, y, 7), block);
                fill(f, canvas, (7, y, 0), (7, y, 7), block);
                fill(f, canvas, (1, y, 0), (6, y, 0), block);
                fill(f, canvas, (1, y, 7), (6, y, 7), block);
            }
            for (from, to) in [((0, 1, 3), (0, 2, 4)), ((7, 1, 3), (7, 2, 4)), ((3, 1, 0), (4, 2, 0)), ((3, 1, 7), (4, 2, 7))] {
                fill(f, canvas, from, to, DARK);
            }
            for (face, from, to) in
                [(SOUTH, (3, 1, 0), (4, 2, 0)), (NORTH, (3, 1, 7), (4, 2, 7)), (WEST, (0, 1, 3), (0, 2, 4)), (EAST, (7, 1, 3), (7, 2, 4))]
            {
                if open[face] {
                    water_box(f, canvas, from, to);
                }
            }
        }
        _ => {}
    }
    if pillar {
        fill(f, canvas, (3, 1, 3), (4, 1, 4), BRICKS);
        fill(f, canvas, (3, 2, 3), (4, 2, 4), PRISMARINE);
        fill(f, canvas, (3, 3, 3), (4, 3, 4), BRICKS);
    }
}

pub fn simple_top(f: &Frame, piece: &Piece, canvas: &mut Canvas, random: &mut Mt19937) {
    floor_and_ceiling(f, piece, canvas);
    for x in 1..=6 {
        for z in 1..=6 {
            if random.next_int(3) != 0 {
                let y = 2 + i32::from(random.next_int(4) != 0);
                fill(f, canvas, (x, y, z), (x, 3, z), Block::WetSponge);
            }
        }
    }
    for (y, block) in [(1, BRICKS), (2, DARK), (3, BRICKS)] {
        fill(f, canvas, (0, y, 0), (0, y, 7), block);
        fill(f, canvas, (7, y, 0), (7, y, 7), block);
        fill(f, canvas, (1, y, 0), (6, y, 0), block);
        fill(f, canvas, (1, y, 7), (6, y, 7), block);
    }
    for (from, to) in [((0, 1, 3), (0, 2, 4)), ((7, 1, 3), (7, 2, 4)), ((3, 1, 0), (4, 2, 0)), ((3, 1, 7), (4, 2, 7))] {
        fill(f, canvas, from, to, DARK);
    }
    if piece.rooms[0].open[SOUTH] {
        water_box(f, canvas, (3, 1, 0), (4, 2, 0));
    }
}

pub fn double_y(f: &Frame, piece: &Piece, canvas: &mut Canvas) {
    let (low, up) = (piece.rooms[0], piece.rooms[1]);
    if piece.index / 25 > 0 {
        default_floor(f, canvas, 0, 0, low.open[DOWN]);
    }
    if !up.linked_up {
        box_on_water(f, canvas, (1, 8, 1), (6, 8, 6), PRISMARINE);
    }
    for (from, to) in [
        ((0, 0), (0, 7)),
        ((7, 0), (7, 7)),
        ((1, 0), (6, 0)),
        ((1, 7), (6, 7)),
        ((2, 1), (2, 2)),
        ((1, 2), (1, 2)),
        ((5, 1), (5, 2)),
        ((6, 2), (6, 2)),
        ((2, 5), (2, 6)),
        ((1, 5), (1, 5)),
        ((5, 5), (5, 6)),
        ((6, 5), (6, 5)),
    ] {
        fill(f, canvas, (from.0, 4, from.1), (to.0, 4, to.1), BRICKS);
    }
    for (y, room) in [(1, low), (5, up)] {
        for (face, z) in [(SOUTH, 0), (NORTH, 7)] {
            if room.open[face] {
                fill(f, canvas, (2, y, z), (2, y + 2, z), BRICKS);
                fill(f, canvas, (5, y, z), (5, y + 2, z), BRICKS);
                fill(f, canvas, (3, y + 2, z), (4, y + 2, z), BRICKS);
            } else {
                fill(f, canvas, (0, y, z), (7, y + 2, z), BRICKS);
                fill(f, canvas, (0, y + 1, z), (7, y + 1, z), PRISMARINE);
            }
        }
        for (face, x) in [(WEST, 0), (EAST, 7)] {
            if room.open[face] {
                fill(f, canvas, (x, y, 2), (x, y + 2, 2), BRICKS);
                fill(f, canvas, (x, y, 5), (x, y + 2, 5), BRICKS);
                fill(f, canvas, (x, y + 2, 3), (x, y + 2, 4), BRICKS);
            } else {
                fill(f, canvas, (x, y, 0), (x, y + 2, 7), BRICKS);
                fill(f, canvas, (x, y + 1, 0), (x, y + 1, 7), PRISMARINE);
            }
        }
    }
}

pub fn double_x(f: &Frame, piece: &Piece, canvas: &mut Canvas) {
    let (west, east) = (piece.rooms[0], piece.rooms[1]);
    if piece.index / 25 > 0 {
        default_floor(f, canvas, 8, 0, east.open[DOWN]);
        default_floor(f, canvas, 0, 0, west.open[DOWN]);
    }
    if !west.linked_up {
        box_on_water(f, canvas, (1, 4, 1), (7, 4, 6), PRISMARINE);
    }
    if !east.linked_up {
        box_on_water(f, canvas, (8, 4, 1), (14, 4, 6), PRISMARINE);
    }
    for (y, block) in [(3, BRICKS), (2, PRISMARINE), (1, BRICKS)] {
        fill(f, canvas, (0, y, 0), (0, y, 7), block);
        fill(f, canvas, (15, y, 0), (15, y, 7), block);
        fill(f, canvas, (1, y, 0), (15, y, 0), block);
        fill(f, canvas, (1, y, 7), (14, y, 7), block);
    }
    fill(f, canvas, (5, 1, 0), (10, 1, 4), BRICKS);
    fill(f, canvas, (6, 2, 0), (9, 2, 3), PRISMARINE);
    fill(f, canvas, (5, 3, 0), (10, 3, 4), BRICKS);
    f.place(canvas, LAMP, 6, 2, 3);
    f.place(canvas, LAMP, 9, 2, 3);
    for (open, from, to) in [
        (west.open[SOUTH], (3, 1, 0), (4, 2, 0)),
        (west.open[NORTH], (3, 1, 7), (4, 2, 7)),
        (west.open[WEST], (0, 1, 3), (0, 2, 4)),
        (east.open[SOUTH], (11, 1, 0), (12, 2, 0)),
        (east.open[NORTH], (11, 1, 7), (12, 2, 7)),
        (east.open[EAST], (15, 1, 3), (15, 2, 4)),
    ] {
        if open {
            water_box(f, canvas, from, to);
        }
    }
}
