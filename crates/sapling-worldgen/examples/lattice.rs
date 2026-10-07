//! `lattice <seed> <lattice x> <lattice z>`: the climate at one 8-block biome lattice column and the
//! biome searched at each lattice y, warm-started as BDS does and cold.

use sapling_worldgen::OverworldGenerator;
use sapling_worldgen::biome::overworld_tree;

fn main() {
    let args: Vec<i64> = std::env::args().skip(1).map(|a| a.parse().expect("integer argument")).collect();
    let [seed, lx, lz] = args[..] else { panic!("usage: lattice <seed> <lx> <lz>") };
    let generator = OverworldGenerator::new(seed);
    let climate = generator.density().climate.sample(lx as i32 * 2, lz as i32 * 2);
    println!("{climate:?}");
    let quantize = |v: f32| (v as f64 * 10000.0) as i64;
    let tree = overworld_tree();
    let mut warm = None;
    for ly in -9..=40 {
        let depth = (ly * 8) as f32 * -0.007_812_5 + 1.0 + climate.offset;
        let point = [climate.temperature, climate.humidity, climate.continentalness, climate.erosion, depth, climate.weirdness].map(quantize);
        let leaf = tree.nearest(&point, warm);
        let cold = tree.nearest(&point, None);
        warm = Some(leaf);
        println!("y {:>4} {point:?} -> {} (cold {})", ly * 8, tree.biome(leaf), tree.biome(cold));
    }
}
