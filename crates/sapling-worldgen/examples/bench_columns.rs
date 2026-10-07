//! `bench_columns [seed] [chunk x0] [chunk z0] [side] [reps]` (defaults 20261003 0 0 8 1): time per
//! column of `loaded_blocks` alone and of `chunk_blocks` (everything, what `Generated::load_column`
//! pays), each on a fresh generator sweeping the square x outer, z inner as the oracle does; the best
//! of `reps` sweeps.

use std::time::Instant;

use sapling_worldgen::OverworldGenerator;

fn sweep(seed: i64, x0: i32, z0: i32, side: i32, stage: impl Fn(&OverworldGenerator, i32, i32)) -> f64 {
    let generator = OverworldGenerator::new(seed);
    let start = Instant::now();
    for x in x0..x0 + side {
        for z in z0..z0 + side {
            stage(&generator, x, z);
        }
    }
    start.elapsed().as_secs_f64() * 1000.0 / (side * side) as f64
}

fn main() {
    let args: Vec<i64> = std::env::args().skip(1).map(|a| a.parse().expect("integer argument")).collect();
    let arg = |i: usize, default: i64| args.get(i).copied().unwrap_or(default);
    let (seed, x0, z0, side) = (arg(0, 20261003), arg(1, 0) as i32, arg(2, 0) as i32, arg(3, 8) as i32);
    let reps = arg(4, 1);
    let best = |stage: &dyn Fn(&OverworldGenerator, i32, i32)| (0..reps).map(|_| sweep(seed, x0, z0, side, stage)).fold(f64::MAX, f64::min);
    println!("loaded_blocks: {:.2} ms/column", best(&|g, x, z| drop(g.loaded_blocks(x, z))));
    println!("chunk_blocks:  {:.2} ms/column", best(&|g, x, z| drop(g.chunk_blocks(x, z))));
}
