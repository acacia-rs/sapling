//! `v2_pieces <seed> <chunk x> <chunk z>`: the pieces of every v2 jigsaw structure (trail ruins,
//! trial chambers, abandoned camps) with a piece in the chunk, in assembly order: element, origin,
//! rotation, box, depth. Beside a BDS log of `FeaturePoolElement` and template placements.

use sapling_worldgen::OverworldGenerator;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, cx, cz] = &args[..] else { panic!("usage: v2_pieces <seed> <cx> <cz>") };
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    generator.v2_pieces(cx.parse().unwrap(), cz.parse().unwrap()).iter().for_each(|line| println!("{line}"));
}
