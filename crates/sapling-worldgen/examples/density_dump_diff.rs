//! `density_dump_diff <seed> <cx> <cz> <dump.bin> [<main.bin> <min.bin> <max.bin>]`: BDS's cell-corner
//! densities of a chunk (the output buffer of `_computeDensityCells` 14822d4b0, dumped by
//! scripts/bds_dbg.py's `dump_on_return`) against ours: how many corners differ, by how many f32
//! steps, and the first of them. The three optional files are the blended noise's octave sums (the
//! function's stack buffers at rsp+0x213c, +0x1138 and +0x134 once the fills are done, 14822d84a).
//! `density_dump_diff <seed> dir <dir>` compares every `dump-<n>-cells-<cx>_<cz>.bin` of a debugger
//! run, and beside each its climate cache `ptr-cells-<cx>_<cz>-0.bin` (8x8 quart columns from
//! chunk * 4 - 2, 0x28 bytes each), printing the chunks that differ and the totals.

use sapling_worldgen::OverworldGenerator;
use sapling_worldgen::density::{Density, MIN_CELL_Y, cell_index};

const CACHE_WIDTH: i32 = 8;
const CACHE_ENTRY: usize = 0x28;

fn floats(path: &str) -> Vec<f32> {
    let bytes = std::fs::read(path).expect("dump file");
    bytes.chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect()
}

/// How many values differ and by how many f32 steps at most; prints up to `show` of them.
fn compare(name: &str, ours: &[f32], bds: &[f32], show: usize) -> (usize, i64) {
    let (mut differing, mut worst) = (0, 0i64);
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
                if differing <= show {
                    println!("{name} corner {x} {z} row {y} (y {}): ours {:e} bds {:e} ({steps} steps)", (y as i32 + MIN_CELL_Y) * 8, ours[i], bds[i]);
                }
            }
        }
    }
    (differing, worst)
}

fn densities(density: &Density, cx: i32, cz: i32) -> Vec<f32> {
    let climate = |qx: i32, qz: i32| density.climate.sample(qx, qz);
    density.cells(cx, cz, &climate).density.to_vec()
}

/// The climate cache's values that differ from ours, of 640.
fn climate_differences(density: &Density, cx: i32, cz: i32, bds: &[f32]) -> usize {
    let mut differing = 0;
    for dx in 0..CACHE_WIDTH {
        for dz in 0..CACHE_WIDTH {
            let c = density.climate.sample(cx * 4 - 2 + dx, cz * 4 - 2 + dz);
            let ours = [c.shifted_x, c.shifted_z, c.continentalness, c.weirdness, c.erosion, c.offset, c.factor, c.jagged, c.temperature, c.humidity];
            let at = (dx * CACHE_WIDTH + dz) as usize * CACHE_ENTRY / 4;
            differing += (0..10).filter(|&k| ours[k].to_bits() != bds[at + k].to_bits()).count();
        }
    }
    differing
}

fn directory(density: &Density, dir: &str) {
    let (mut chunks, mut corners, mut climates) = (0, 0, 0);
    for entry in std::fs::read_dir(dir).expect("dump directory").flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(chunk) = name.strip_prefix("dump-").and_then(|n| n.split_once("-cells-")).and_then(|(_, c)| c.strip_suffix(".bin")) else { continue };
        let Some((cx, cz)) = chunk.split_once('_').and_then(|(x, z)| Some((x.parse::<i32>().ok()?, z.parse::<i32>().ok()?))) else { continue };
        let (differing, worst) = compare("", &densities(density, cx, cz), &floats(&format!("{dir}/{name}")), 0);
        let cache = format!("{dir}/ptr-cells-{chunk}-0.bin");
        let climate = if std::path::Path::new(&cache).exists() { climate_differences(density, cx, cz, &floats(&cache)) } else { 0 };
        chunks += 1;
        corners += differing;
        climates += climate;
        if differing + climate > 0 {
            println!("chunk {cx} {cz}: {differing} corners differ (at most {worst} f32 steps), {climate} climate values");
        }
    }
    println!("{chunks} chunks: {corners} of {} corner densities differ, {climates} climate values", chunks * 1025);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let [seed, mode, dir] = &args[..]
        && mode == "dir"
    {
        return directory(OverworldGenerator::new(seed.parse().unwrap()).density(), dir);
    }
    let [seed, cx, cz, path, parts @ ..] = &args[..] else { panic!("usage: density_dump_diff <seed> <cx> <cz> <dump.bin> [main min max] | <seed> dir <dir>") };
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    let density = generator.density();
    let (cx, cz): (i32, i32) = (cx.parse().unwrap(), cz.parse().unwrap());
    let bds = floats(path);
    let (differing, worst) = compare("density", &densities(density, cx, cz), &bds, 30);
    println!("density: {differing} of {} corners differ, at most {worst} f32 steps", bds.len());
    for ((name, file), ours) in ["main", "min", "max"].iter().zip(parts).zip(&density.blended_parts(cx, cz)) {
        let (differing, worst) = compare(name, &ours[..], &floats(file), 30);
        println!("{name}: {differing} of 1025 corners differ, at most {worst} f32 steps");
    }
}
