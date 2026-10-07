//! `timing <seed> <x0> <z0> <x1> <z1>`: milliseconds per column for `chunk_blocks` (every ported
//! stage) over a chunk range, one generator, cold caches.

use std::time::Instant;

use sapling_worldgen::OverworldGenerator;

fn main() {
    let args: Vec<i64> = std::env::args().skip(1).map(|a| a.parse().unwrap()).collect();
    let [seed, x0, z0, x1, z1] = args[..] else { panic!("usage: timing <seed> x0 z0 x1 z1") };
    let generator = OverworldGenerator::new(seed);
    let start = Instant::now();
    let mut columns = 0;
    for x in x0 as i32..=x1 as i32 {
        for z in z0 as i32..=z1 as i32 {
            std::hint::black_box(generator.chunk_blocks(x, z));
            columns += 1;
        }
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / columns as f64;
    println!("{columns} columns, {ms:.2} ms per column");
}
