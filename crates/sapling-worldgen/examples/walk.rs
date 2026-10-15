//! `walk <seed> <length> <width> <path> [areas]`: a long walk on one thread, a band of `width` chunks
//! (z from 0) generated column by column along x, as a player walking east loads it. `path`: `out`
//! (x 0..length), `in` (the same band from the far end back), `outback` (out, then in again, every
//! column generated twice). Every 1000 columns: peak and current RSS and the structure start maps'
//! entries. Ends with ms per column and a hash of every block, summed per column so that it does not
//! depend on the order (tools/bench.sh `walk*`).

use std::hash::{DefaultHasher, Hash, Hasher};
use std::time::Instant;

use sapling_worldgen::OverworldGenerator;

fn column_hash(generator: &OverworldGenerator, x: i32, z: i32) -> u64 {
    let shape = generator.chunk_blocks(x, z);
    let mut hasher = DefaultHasher::new();
    (x, z).hash(&mut hasher);
    for y in sapling_worldgen::MIN_Y..sapling_worldgen::MIN_Y + sapling_worldgen::HEIGHT as i32 {
        for z in 0..16 {
            for x in 0..16 {
                shape.block(x, y, z).hash(&mut hasher);
            }
        }
    }
    hasher.finish()
}

/// VmHWM and VmRSS from Linux's /proc, in MB (0 elsewhere).
fn rss_mb() -> (u64, u64) {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let field = |name: &str| status.lines().find_map(|l| l.strip_prefix(name)).and_then(|v| v.trim().trim_end_matches("kB").trim().parse::<u64>().ok());
    (field("VmHWM:").unwrap_or(0) / 1024, field("VmRSS:").unwrap_or(0) / 1024)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, length, width, path] = &args[..4] else { panic!("usage: walk <seed> <length> <width> out|in|outback [areas]") };
    let (seed, length, width): (i64, i32, i32) = (seed.parse().unwrap(), length.parse().unwrap(), width.parse().unwrap());
    let areas = args.get(4).map_or(1, |a| a.parse().unwrap());
    let generator = OverworldGenerator::with_areas(seed, areas);
    let out: Vec<i32> = (0..length).collect();
    let back: Vec<i32> = (0..length).rev().collect();
    let passes: Vec<&[i32]> = match path.as_str() {
        "out" => vec![&out],
        "in" => vec![&back],
        "outback" => vec![&out, &back],
        _ => panic!("path: out, in or outback"),
    };
    let start = Instant::now();
    let mut columns = 0u64;
    for (pass, xs) in passes.iter().enumerate() {
        let mut sum = 0u64;
        for &x in xs.iter() {
            for z in 0..width {
                sum = sum.wrapping_add(column_hash(&generator, x, z));
                columns += 1;
                if columns % 1000 == 0 {
                    let (peak, now) = rss_mb();
                    let counts: Vec<String> = generator.structure_cache_counts().iter().map(|(name, n)| format!("{name} {n}")).collect();
                    println!("columns {columns} rss {now} MB peak {peak} MB | {}", counts.join(", "));
                }
            }
        }
        println!("pass {pass} hash {sum:016x}");
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / columns as f64;
    println!("{columns} columns, {ms:.2} ms per column");
    println!("allocations 0 MB 0 hash -");
}
