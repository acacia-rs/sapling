//! `adjustment <seed> <scale> <y> <x> <z>...`: the surface material-adjustment noise at columns.

use sapling_worldgen::OverworldGenerator;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let seed: i64 = args[0].parse().unwrap();
    let scale: f32 = args[1].parse().unwrap();
    let y: i32 = args[2].parse().unwrap();
    let generator = OverworldGenerator::new(seed);
    for pair in args[3..].chunks(2) {
        let (x, z): (i32, i32) = (pair[0].parse().unwrap(), pair[1].parse().unwrap());
        println!("{x} {z}: {:.4}", generator.adjustment_noise([x, y, z], scale));
    }
}
