//! `density_dump_diff <seed> <cx> <cz> <dump.bin>`: BDS's cell-corner densities of a chunk (the output
//! buffer of `_computeDensityCells` 14822d4b0, dumped by scripts/bds_dbg.py's `dump_on_return`) against
//! ours: how many corners differ, by how many f32 steps, and the first of them.

use sapling_worldgen::OverworldGenerator;
use sapling_worldgen::density::{MIN_CELL_Y, cell_index};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, cx, cz, path] = &args[..] else { panic!("usage: density_dump_diff <seed> <cx> <cz> <dump.bin>") };
    let (cx, cz): (i32, i32) = (cx.parse().unwrap(), cz.parse().unwrap());
    let bytes = std::fs::read(path).expect("dump file");
    let bds: Vec<f32> = bytes.chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect();
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    let density = generator.density();
    let climate = |qx: i32, qz: i32| density.climate.sample(qx, qz);
    let cells = density.cells(cx, cz, &climate);
    let (mut differing, mut worst, mut shown) = (0, 0i64, 0);
    for x in 0..5 {
        for z in 0..5 {
            for y in 0..41 {
                let i = cell_index(x, z, y);
                let (ours, theirs) = (cells.density[i], bds[i]);
                if ours.to_bits() == theirs.to_bits() {
                    continue;
                }
                let steps = (ours.to_bits() as i32 as i64 - theirs.to_bits() as i32 as i64).abs();
                differing += 1;
                worst = worst.max(steps);
                if shown < 40 {
                    shown += 1;
                    println!("corner {x} {z} row {y} (y {}): ours {ours:e} bds {theirs:e} ({steps} steps)", (y as i32 + MIN_CELL_Y) * 8);
                }
            }
        }
    }
    println!("{differing} of {} corners differ, at most {worst} f32 steps", bds.len());
}
