//! `point_biome <seed> <x> <y> <z>`: the biome BDS's point query gives a block (no lattice), and
//! the preliminary surface of its quart column.

use sapling_worldgen::OverworldGenerator;

fn main() {
    let args: Vec<i64> = std::env::args().skip(1).map(|a| a.parse().unwrap()).collect();
    let [seed, x, y, z] = args[..] else { panic!("usage: point_biome <seed> <x> <y> <z>") };
    let generator = OverworldGenerator::new(seed);
    let density = generator.density();
    let (x, y, z) = (x as i32, y as i32, z as i32);
    println!("biome {} surface {}", density.climate.biome_at(x, y, z), density.preliminary_surface(x >> 2, z >> 2));
}
