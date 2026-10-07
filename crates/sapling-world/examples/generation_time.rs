//! `generation_time [seed] [side] [world folder]`: milliseconds per generated column on one
//! thread, by stage, over `side` x `side` columns from (0, 0); with a folder, also per column
//! loaded from its database. Run in release:
//! `cargo run --release -p sapling-world --example generation_time -- 20261003 8 <folder>`.

use std::path::Path;
use std::time::{Duration, Instant};

use sapling_world::{ChunkPos, Generated, LevelDbStorage, MemoryStorage, WorldStorage};
use sapling_worldgen::OverworldGenerator;

fn timed<T>(total: &mut Duration, run: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let value = run();
    *total += start.elapsed();
    value
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let seed = args.first().map_or(20261003, |a| a.parse().expect("seed"));
    let side: i32 = args.get(1).map_or(8, |a| a.parse().expect("side"));
    let positions: Vec<ChunkPos> = (0..side).flat_map(|x| (0..side).map(move |z| ChunkPos::new(x, z))).collect();
    let per_column = |total: Duration| total.as_secs_f64() * 1000.0 / positions.len() as f64;

    let mut setup = Duration::ZERO;
    let generated = timed(&mut setup, || Generated::new(MemoryStorage::default(), seed));
    let generator = OverworldGenerator::new(seed);
    println!("seed {seed}: generator built in {:.1} ms; {} columns", setup.as_secs_f64() * 1000.0, positions.len());

    let [mut surfaced, mut carved, mut decorated, mut featured, mut biomes, mut whole] = [Duration::ZERO; 6];
    for &pos in &positions {
        let mut shape = timed(&mut surfaced, || generator.surfaced_blocks(pos.x, pos.z));
        timed(&mut carved, || generator.carve(&mut shape, pos.x, pos.z));
        timed(&mut decorated, || generator.decorate(&mut shape, pos.x, pos.z));
        timed(&mut featured, || generator.place_features(&mut shape, pos.x, pos.z));
        timed(&mut biomes, || generator.chunk_biomes(pos.x, pos.z));
        timed(&mut whole, || generated.load_column(pos, 0).expect("generated"));
    }
    let stages = surfaced + carved + decorated + featured + biomes;
    println!("  surfaced blocks {:.2} ms (shape, floor, surface)", per_column(surfaced));
    println!("  carvers         {:.2} ms", per_column(carved));
    println!("  lava lakes      {:.2} ms (neighbours' terrain generated once, cached)", per_column(decorated));
    println!("  features        {:.2} ms (with the neighbours' terrain and decoration they need)", per_column(featured));
    println!("  biomes          {:.2} ms", per_column(biomes));
    println!("  to a Chunk      {:.2} ms (whole minus the stages)", per_column(whole.saturating_sub(stages)));
    println!("  whole column    {:.2} ms through Generated::load_column", per_column(whole));

    if let Some(folder) = args.get(2) {
        let storage = LevelDbStorage::open_in_memory(Path::new(folder)).expect("the folder opens");
        let mut loaded = Duration::ZERO;
        let found = positions.iter().filter(|&&pos| timed(&mut loaded, || storage.load_column(pos, 0).expect("readable")).is_some()).count();
        println!("  from LevelDB    {:.2} ms per column ({found} of {} stored)", per_column(loaded), positions.len());
    }
}
