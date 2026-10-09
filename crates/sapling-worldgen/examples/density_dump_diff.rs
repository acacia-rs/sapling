//! `density_dump_diff <seed> <cx> <cz> <dump.bin> [<main.bin> <min.bin> <max.bin>]`: BDS's cell-corner
//! densities of a chunk (the output buffer of `_computeDensityCells` 14822d4b0, dumped by
//! scripts/bds_dbg.py's `dump_on_return`) against ours: how many corners differ, by how many f32
//! steps, and the first of them. The three optional files are the blended noise's octave sums (the
//! function's stack buffers at rsp+0x213c, +0x1138 and +0x134 once the fills are done, 14822d84a).

use sapling_worldgen::OverworldGenerator;
use sapling_worldgen::density::{MIN_CELL_Y, cell_index};

fn floats(path: &str) -> Vec<f32> {
    let bytes = std::fs::read(path).expect("dump file");
    bytes.chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect()
}

fn compare(name: &str, ours: &[f32], bds: &[f32]) {
    let (mut differing, mut worst, mut shown) = (0, 0i64, 0);
    for x in 0..5 {
        for z in 0..5 {
            for y in 0..41 {
                let i = cell_index(x, z, y);
                if ours[i].to_bits() == bds[i].to_bits() {
                    continue;
                }
                let steps = (ours[i].to_bits() as i32 as i64 - bds[i].to_bits() as i32 as i64).abs();
                differing += 1;
                worst = worst.max(steps);
                if shown < 30 {
                    shown += 1;
                    println!("{name} corner {x} {z} row {y} (y {}): ours {:e} bds {:e} ({steps} steps)", (y as i32 + MIN_CELL_Y) * 8, ours[i], bds[i]);
                }
            }
        }
    }
    println!("{name}: {differing} of {} corners differ, at most {worst} f32 steps", bds.len());
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, cx, cz, path, parts @ ..] = &args[..] else { panic!("usage: density_dump_diff <seed> <cx> <cz> <dump.bin> [main min max]") };
    let (cx, cz): (i32, i32) = (cx.parse().unwrap(), cz.parse().unwrap());
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    let density = generator.density();
    let climate = |qx: i32, qz: i32| density.climate.sample(qx, qz);
    compare("density", &density.cells(cx, cz, &climate).density[..], &floats(path));
    let ours = density.blended_parts(cx, cz);
    for ((name, file), ours) in ["main", "min", "max"].iter().zip(parts).zip(&ours) {
        compare(name, &ours[..], &floats(file));
    }
}
