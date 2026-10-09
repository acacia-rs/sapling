//! `pieced_column <seed> <x> <z> <y0> <y1>`: a column's blocks as the chunk's pieces place them,
//! before connections are joined (walls and fences as their templates have them), top down.

use sapling_worldgen::OverworldGenerator;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, x, z, y0, y1] = &args[..] else { panic!("usage: pieced_column <seed> <x> <z> <y0> <y1>") };
    let [x, z, y0, y1] = [x, z, y0, y1].map(|v| v.parse::<i32>().unwrap());
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    let shape = generator.pieced_uncached(x >> 4, z >> 4);
    for y in (y0..=y1).rev() {
        let block = shape.block((x & 15) as usize, y, (z & 15) as usize);
        println!("{x} {y} {z} {}[{}]", block.name(), block.states());
    }
}
