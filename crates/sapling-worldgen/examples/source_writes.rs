//! `source_writes <seed> <chunk x> <chunk z>`: every feature write of one source chunk decorated
//! alone, in order, as `x y z block replaced`; to set beside a BDS breakpoint log.

use sapling_worldgen::OverworldGenerator;

fn main() {
    let args: Vec<i64> = std::env::args().skip(1).map(|a| a.parse().expect("integer argument")).collect();
    let [seed, cx, cz] = args[..] else { panic!("usage: source_writes <seed> <cx> <cz>") };
    let generator = OverworldGenerator::new(seed);
    for ([x, y, z], block, replaced) in generator.source_feature_writes(cx as i32, cz as i32) {
        println!("{x} {y} {z} {block:?} {replaced:?}");
    }
}
