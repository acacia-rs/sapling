//! `bench_columns <seed> <chunk x0> <chunk z0> <side>`: time per column of `loaded_blocks` alone
//! and of `chunk_blocks` (everything, what `Generated::load_column` pays), each on a fresh
//! generator sweeping the square x outer, z inner as the oracle does.

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
    let [seed, x0, z0, side] = args[..] else { panic!("usage: bench_columns <seed> <x0> <z0> <side>") };
    let (x0, z0, side) = (x0 as i32, z0 as i32, side as i32);
    let loaded = sweep(seed, x0, z0, side, |g, x, z| drop(g.loaded_blocks(x, z)));
    println!("loaded_blocks: {loaded:.2} ms/column");
    let all = sweep(seed, x0, z0, side, |g, x, z| drop(g.chunk_blocks(x, z)));
    println!("chunk_blocks:  {all:.2} ms/column");
}
