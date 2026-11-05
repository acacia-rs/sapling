//! `spring_find <seed> <cx0> <cz0> <cx1> <cz1>`: every sulfur spring the chunks' decoration places, with its
//! template, rotation and the block under it, from the decoration trace. For choosing spring canon regions.

use sapling_worldgen::OverworldGenerator;

fn main() {
    let args: Vec<i64> = std::env::args().skip(1).map(|arg| arg.parse().expect("a number")).collect();
    let [seed, x0, z0, x1, z1] = args[..] else { panic!("usage: spring_find <seed> cx0 cz0 cx1 cz1") };
    let generator = OverworldGenerator::new(seed);
    for cx in x0 as i32..=x1 as i32 {
        for cz in z0 as i32..=z1 as i32 {
            for line in generator.source_feature_trace(cx, cz).iter().filter(|line| line.contains("sulfur spring")) {
                println!("{cx} {cz}{line}");
            }
        }
    }
}
