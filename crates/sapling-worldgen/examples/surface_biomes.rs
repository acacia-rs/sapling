//! `surface_biomes <seed> <cx0> <cz0> <cx1> <cz1>`: how many columns each biome tops, by our
//! generator, to see which surface builders a region exercises.

use std::collections::BTreeMap;

use sapling_worldgen::{MIN_Y, OverworldGenerator};

fn main() {
    let args: Vec<i64> = std::env::args().skip(1).map(|a| a.parse().expect("integer argument")).collect();
    let [seed, x0, z0, x1, z1] = args[..] else { panic!("usage: surface_biomes <seed> <cx0> <cz0> <cx1> <cz1>") };
    let generator = OverworldGenerator::new(seed);
    let mut counts: BTreeMap<u32, u64> = BTreeMap::new();
    for cx in x0 as i32..=x1 as i32 {
        for cz in z0 as i32..=z1 as i32 {
            let shape = generator.chunk_shape(cx, cz);
            let biomes = generator.chunk_biomes(cx, cz);
            for x in 0..16 {
                for z in 0..16 {
                    let top = (MIN_Y..=255).rev().find(|&y| shape.kind(x, y, z) != sapling_worldgen::CellKind::Air).unwrap_or(MIN_Y);
                    *counts.entry(biomes.biome(x, top + 1, z)).or_default() += 1;
                }
            }
        }
    }
    println!("{counts:?}");
}
