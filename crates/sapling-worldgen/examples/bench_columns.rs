//! `bench_columns [seed] [chunks]`: wall time per whole column (`chunk_blocks`) over a square of
//! chunks from 0,0, on one fresh generator (neighbour caches amortised as when generating an area).

use std::time::Instant;

use sapling_worldgen::OverworldGenerator;

fn main() {
    let mut args = std::env::args().skip(1);
    let seed: i64 = args.next().map_or(20261003, |a| a.parse().expect("seed"));
    let side: i32 = args.next().map_or(8, |a| a.parse().expect("chunk count per side"));
    let generator = OverworldGenerator::new(seed);
    let start = Instant::now();
    let mut solid = 0usize;
    for cx in 0..side {
        for cz in 0..side {
            let shape = generator.chunk_blocks(cx, cz);
            solid += usize::from(shape.block(0, 0, 0).is_solid());
        }
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(side * side);
    println!("{:.2} ms per column over {} columns ({solid} solid at y 0)", ms, side * side);
}
