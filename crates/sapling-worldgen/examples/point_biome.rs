//! `point_biome <seed> <x> <y> <z> [<x> <y> <z>...]`: the biome BDS's point query gives each block (no
//! lattice), and the preliminary surface of its quart column; one line per point.

use sapling_worldgen::OverworldGenerator;

fn main() {
    let args: Vec<i64> = std::env::args().skip(1).map(|a| a.parse().unwrap()).collect();
    let [seed, ref points @ ..] = args[..] else { panic!("usage: point_biome <seed> <x> <y> <z> [<x> <y> <z>...]") };
    assert!(!points.is_empty() && points.len() % 3 == 0, "usage: point_biome <seed> <x> <y> <z> [<x> <y> <z>...]");
    let generator = OverworldGenerator::new(seed);
    let density = generator.density();
    for point in points.chunks(3) {
        let (x, y, z) = (point[0] as i32, point[1] as i32, point[2] as i32);
        println!("{x} {y} {z} biome {} surface {}", density.climate.biome_at(x, y, z), density.preliminary_surface(x >> 2, z >> 2));
    }
}
