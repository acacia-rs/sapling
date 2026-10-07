//! `column <seed> <block x> <block z>`: one generated column as runs of equal blocks, top
//! down, to set beside `worldgen-oracle <world> <x> <z>`.

use sapling_worldgen::{MIN_Y, OverworldGenerator};

fn main() {
    let args: Vec<i64> = std::env::args().skip(1).map(|a| a.parse().expect("integer argument")).collect();
    let [seed, x, z] = args[..] else { panic!("usage: column <seed> <x> <z>") };
    let (x, z) = (x as i32, z as i32);
    let generator = OverworldGenerator::new(seed);
    let shape = if std::env::var_os("UNCARVED").is_some() {
        generator.surfaced_blocks(x >> 4, z >> 4)
    } else {
        generator.chunk_blocks(x >> 4, z >> 4)
    };
    let kind = |y| shape.block((x & 15) as usize, y, (z & 15) as usize);
    let mut top = 319;
    for y in (MIN_Y..=319).rev() {
        if y == MIN_Y || kind(y - 1) != kind(y) {
            println!("{y:>4}..={top:<4} {:?}", kind(y));
            top = y - 1;
        }
    }
}
