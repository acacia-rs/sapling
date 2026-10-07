//! `find_biomes <seed> <radius in chunks> <biome id>...`: for each biome, the 8x8-chunk tile
//! (aligned to 8) within the radius whose surface it covers most, sampling every other chunk at
//! the preliminary surface. For choosing oracle regions (tools/worldgen-oracle/run.sh).

use std::collections::HashMap;

use sapling_worldgen::OverworldGenerator;
use sapling_worldgen::biome::overworld_tree;

fn main() {
    let args: Vec<i64> = std::env::args().skip(1).map(|a| a.parse().expect("integer argument")).collect();
    let (seed, radius, targets) = (args[0], args[1] as i32, &args[2..]);
    let generator = OverworldGenerator::new(seed);
    let density = generator.density();
    let mut tiles: HashMap<(u32, i32, i32), u32> = HashMap::new();
    for cx in (-radius..radius).step_by(2) {
        for cz in (-radius..radius).step_by(2) {
            let (qx, qz) = (cx * 4 + 2, cz * 4 + 2);
            let surface = density.preliminary_surface(qx, qz).min(256);
            let climate = density.climate.sample(qx, qz);
            let depth = (surface + 8) as f32 * -0.007_812_5 + 1.0 + climate.offset;
            let point = [climate.temperature, climate.humidity, climate.continentalness, climate.erosion, depth, climate.weirdness]
                .map(|v| (v as f64 * 10000.0) as i64);
            let tree = overworld_tree();
            let biome = tree.biome(tree.nearest(&point, None));
            *tiles.entry((biome, cx.div_euclid(8), cz.div_euclid(8))).or_default() += 1;
        }
    }
    for &target in targets {
        let best = tiles.iter().filter(|((b, _, _), _)| *b as i64 == target).max_by_key(|(_, n)| **n);
        match best {
            Some(((_, tx, tz), n)) => println!("{target}: {n}/16 at {},{},{},{}", tx * 8, tz * 8, tx * 8 + 7, tz * 8 + 7),
            None => println!("{target}: none"),
        }
    }
}
