//! The desert pyramid's cellar (the tail of 148360db0, Java 1.20's `addCellar`): stairs down from
//! the west room, a sand-filled room, and suspicious sand in a shuffled few of its cells.

use crate::block::Block;
use crate::random::{Mt19937, Xoroshiro, position_seed};
use crate::structure::canvas::Canvas;
use crate::structure::piece::Frame;

pub fn place(f: &Frame, canvas: &mut Canvas, random: &mut Mt19937) {
    let (sand, sandstone, cut, chiseled, orange) =
        (Block::Sand, Block::Sandstone, Block::CutSandstone, Block::ChiseledSandstone, Block::OrangeTerracotta);
    for (x, y) in [(13, -1), (14, -2), (15, -3)] {
        f.place(canvas, Block::SandstoneStairsWest, x, y, 17);
    }
    let sand_first = random.next_boolean();
    for x in 12..=16 {
        f.place(canvas, sand, x, 0, 17);
    }
    f.place(canvas, sand, 14, -1, 17);
    f.place(canvas, if sand_first { sand } else { sandstone }, 15, -1, 17);
    f.place(canvas, if sand_first { sandstone } else { sand }, 16, -1, 17);
    f.place(canvas, sand, 15, -2, 17);
    f.place(canvas, sandstone, 16, -2, 17);
    f.place(canvas, sand, 16, -3, 17);
    for (y, wall) in [(-3, cut), (-2, chiseled), (-1, cut)] {
        for (from, to) in [((13, y, 10), (13, y, 15)), ((19, y, 10), (19, y, 15)), ((13, y, 10), (19, y, 11)), ((13, y, 16), (19, y, 16))] {
            f.generate_box(canvas, from, to, wall, wall, true);
        }
    }
    // A Java Xoroshiro on the room's corner, its seed truncated to 32 bits after the mix.
    let (x, y, z) = f.world(16, -4, 13);
    let mut xoroshiro = Xoroshiro::from_seed(position_seed(x, y, z) as i32 as i64);
    let mut cells = Vec::new();
    for y in -3..=-1 {
        for x in 14..=18 {
            for z in 11..=15 {
                f.place(canvas, sand, x, y, z);
                cells.push((x, y, z));
            }
        }
    }
    for x in 14..=18 {
        for z in 11..=15 {
            let block = if xoroshiro.next_int_bounded(3) != 0 { sand } else { sandstone };
            f.place(canvas, block, x, 0, z);
        }
    }
    f.place(canvas, Block::BlueTerracotta, 16, -4, 13);
    for (x, z) in [(17, 14), (17, 12), (15, 14), (15, 12), (18, 13), (14, 13), (16, 15), (16, 11), (19, 13)] {
        f.place(canvas, orange, x, -4, z);
    }
    // Doorways: sand in the wall's gap, the frame around it outside.
    for (x, z, outside) in [(19, 13, Some((20, 13))), (13, 13, Some((12, 13))), (16, 16, None), (16, 10, Some((16, 9)))] {
        if (x, z) != (19, 13) {
            f.place(canvas, orange, x, -4, z);
        }
        for y in [-3, -2] {
            f.place(canvas, sand, x, y, z);
            cells.push((x, y, z));
        }
        if let Some((ox, oz)) = outside {
            f.place(canvas, cut, ox, -3, oz);
            f.place(canvas, chiseled, ox, -2, oz);
        }
    }
    let count = (xoroshiro.next_int_bounded(3) + 5).min(cells.len() as i32) as usize;
    for i in 1..cells.len() {
        let j = xoroshiro.next_int_bounded(i as u32 + 1) as usize;
        cells.swap(i, j);
    }
    for &cell in &cells[..count] {
        suspicious_sand(f, canvas, &mut xoroshiro, cell);
    }
    let x = xoroshiro.next_int_bounded(5) + 14;
    let z = xoroshiro.next_int_bounded(5) + 11;
    suspicious_sand(f, canvas, &mut xoroshiro, (x, 0, z));
}

/// 1483669e0: suspicious sand, and a loot seed drawn whenever the cell holds its block entity:
/// placed here, or by a chunk post-processed before this one.
fn suspicious_sand(f: &Frame, canvas: &mut Canvas, xoroshiro: &mut Xoroshiro, (x, y, z): (i32, i32, i32)) {
    let (wx, wy, wz) = f.world(x, y, z);
    f.place(canvas, Block::SuspiciousSand, x, y, z);
    if canvas.get(wx, wy, wz) == Block::SuspiciousSand {
        xoroshiro.next_long();
    }
}
