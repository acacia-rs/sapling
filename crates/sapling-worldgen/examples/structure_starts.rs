//! `structure_starts <seed> <x0> <z0> <x1> <z1> [kind]`: every structure start in a chunk range.

use sapling_worldgen::OverworldGenerator;
use sapling_worldgen::structure::StructureKind;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [seed, x0, z0, x1, z1, rest @ ..] = &args[..] else { panic!("usage: structure_starts <seed> x0 z0 x1 z1 [kind]") };
    let n: Vec<i32> = [x0, z0, x1, z1].iter().map(|v| v.parse().unwrap()).collect();
    let generator = OverworldGenerator::new(seed.parse().unwrap());
    for kind in StructureKind::ALL {
        if rest.first().is_some_and(|only| only != kind.name()) {
            continue;
        }
        for x in n[0]..=n[2] {
            for z in n[1]..=n[3] {
                if generator.is_structure_start(kind, x, z) {
                    println!("{} {x} {z}", kind.name());
                }
            }
        }
    }
}
