//! `cell_corners <seed> <block x> <block z> <y0> <y1>`: the density cell corners around a block
//! column, by cell row, for checking how close a shape difference is to the density's zero.

use sapling_worldgen::OverworldGenerator;
use sapling_worldgen::density::{MIN_CELL_Y, cell_index};

fn main() {
    let args: Vec<i32> = std::env::args().skip(2).map(|a| a.parse().expect("integer argument")).collect();
    let seed: i64 = std::env::args().nth(1).expect("seed").parse().unwrap();
    let [x, z, y0, y1] = args[..] else { panic!("usage: cell_corners <seed> <x> <z> <y0> <y1>") };
    let generator = OverworldGenerator::new(seed);
    let density = generator.density();
    let (cx, cz) = (x >> 4, z >> 4);
    let climate = |qx: i32, qz: i32| density.climate.sample(qx, qz);
    let cells = density.cells(cx, cz, &climate);
    let (lx, lz) = (((x & 15) >> 2) as usize, ((z & 15) >> 2) as usize);
    for row in ((y0 >> 3) - MIN_CELL_Y..=(y1 >> 3) - MIN_CELL_Y).rev() {
        let row = row as usize;
        let corners: Vec<String> = [(0, 0), (1, 0), (0, 1), (1, 1)].iter().map(|&(a, b)| format!("{:>10.4}", cells.density[cell_index(lx + a, lz + b, row)])).collect();
        println!("row {row} (y {}): x0z0 x1z0 x0z1 x1z1 {}", (row as i32 + MIN_CELL_Y) * 8, corners.join(" "));
    }
}
