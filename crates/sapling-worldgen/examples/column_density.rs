//! `column_density <seed> (<x> <y> <z>)...`: our block density (before the aquifer) at each cell and
//! the two above and below it, with the cell corners' densities: how close a shape difference is to zero.

use sapling_worldgen::density::{MIN_CELL_Y, cell_index};
use sapling_worldgen::{MIN_Y, OverworldGenerator};

fn main() {
    let args: Vec<i64> = std::env::args().skip(1).map(|a| a.parse().expect("integer argument")).collect();
    let generator = OverworldGenerator::new(args[0]);
    let density = generator.density();
    for cell in args[1..].chunks(3) {
        let (x, y, z) = (cell[0] as i32, cell[1] as i32, cell[2] as i32);
        let column = density.column_density(x, z);
        let near: Vec<String> = (y - 2..=y + 2).map(|cy| format!("{cy}: {:e}", column[(cy - MIN_Y) as usize])).collect();
        let climate = |qx: i32, qz: i32| density.climate.sample(qx, qz);
        let cells = density.cells(x >> 4, z >> 4, &climate);
        let (lx, lz, row) = (((x & 15) >> 2) as usize, ((z & 15) >> 2) as usize, ((y >> 3) - MIN_CELL_Y) as usize);
        let corners: Vec<String> = (0..8).map(|i| format!("{:.5}", cells.density[cell_index(lx + (i & 1), lz + (i >> 1 & 1), row + (i >> 2))])).collect();
        println!("{x} {y} {z} (in cell {} {} {}): {} | corners {}", x & 3, y & 7, z & 3, near.join("  "), corners.join(" "));
    }
}
